#!/usr/bin/env python3
"""Train a tiny test language model on CPU and export it for every backend.

The model exists only to test emdysi's model integrations (the decision
engine and guarded rewriting) end to end where no real model can be
downloaded: it is a few million parameters, trained for minutes on the
vendored corpora with JAX (MLX's CPU backend is far slower for training).
It is written in two formats from the same weights:

- MLX (Hugging Face layout: config.json, model.safetensors, tokenizer.json),
  served by `mlx_lm.server`;
- GGUF, loaded by llama.cpp in-process (`--features llama`) or served by
  `llama-server`.

Besides plain text, it is trained on two toy decisions in the prompt format
of `emdysi_lm::decide` (the letter of an option, after a ChatML chat), so
that the probabilities it returns are not arbitrary:

- which of two spellings is American (pairs from data/scowl/variants.tsv);
- which of two sentences is grammatical (pairs from corpora/blimp).

A tenth of them are held out and written to eval.tsv, for
`en decide --eval`.

Nothing it produces is committed: the output directory defaults to
~/.cache/emdysi/tiny-model. Requires: jax (cpu), optax, safetensors,
tokenizers, gguf, numpy; serving the MLX copy needs mlx-lm (and mlx[cpu]
on Linux).

    python scripts/tiny-model/build.py [--steps 1500] [--out DIR]
"""

import argparse
import json
import pathlib
import random
import time

import gguf
import jax
import jax.numpy as jnp
import numpy as np
import optax
from safetensors.numpy import save_file
from tokenizers import Tokenizer, decoders, models, pre_tokenizers, trainers

ROOT = pathlib.Path(__file__).resolve().parents[2]
SPECIAL = ["<|endoftext|>", "<|im_start|>", "<|im_end|>"]
CHAT_TEMPLATE = (
    "{% for message in messages %}"
    "{{'<|im_start|>' + message['role'] + '\\n' + message['content'] + '<|im_end|>' + '\\n'}}"
    "{% endfor %}"
    "{% if add_generation_prompt %}{{ '<|im_start|>assistant\\n' }}{% endif %}"
)
# Must match emdysi_lm::decide.
SYSTEM = "You make decisions. Reply with only the letter of your choice."
LETTERS = "ABCDEFGHIJ"


def unescape(s):
    out, it = [], iter(s)
    for c in it:
        if c != "\\":
            out.append(c)
            continue
        n = next(it, "")
        out.append({"n": "\n", "t": "\t", "r": "\r"}.get(n, n))
    return "".join(out)


def tsv_rows(path):
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        yield [unescape(c) for c in line.split("\t")]


def texts():
    """Prose from the vendored corpora and documentation."""
    out = []
    for name in ["beemo", "cheat", "hh-rlhf"]:
        for row in tsv_rows(ROOT / "corpora" / name / "sample.tsv"):
            out.extend(c for c in row if len(c.split()) >= 8)
    for p in sorted((ROOT / "corpora" / "ai-prose").glob("*.md")):
        out.append(p.read_text(encoding="utf-8"))
    for p in sorted((ROOT / "docs").glob("*.md")):
        out.append(p.read_text(encoding="utf-8"))
    return out


def chat(user, answer):
    return (
        f"<|im_start|>system\n{SYSTEM}<|im_end|>\n"
        f"<|im_start|>user\n{user}<|im_end|>\n"
        f"<|im_start|>assistant\n{answer}<|im_end|>\n"
    )


def decision_prompt(question, options, context=""):
    # Must match emdysi_lm::decide::prompt.
    lines = [context, ""] if context else []
    lines.append(f"Question: {question}")
    lines += [f"{LETTERS[i]}. {o}" for i, o in enumerate(options)]
    return "\n".join(lines)


def decisions(rng):
    """Toy decision examples: (user message, letter of the right answer)."""
    out = []
    pairs = []
    for row in tsv_rows(ROOT / "data" / "scowl" / "variants.tsv"):
        if len(row) >= 3 and row[1] == "us":
            pairs.append((row[0], row[2]))
    for us, gb in pairs:
        opts = [us, gb]
        rng.shuffle(opts)
        q = "Which spelling is American English?"
        out.append((decision_prompt(q, opts), LETTERS[opts.index(us)]))
    for row in tsv_rows(ROOT / "corpora" / "blimp" / "sample.tsv"):
        if len(row) >= 5:
            good, bad = row[3], row[4]
            opts = [good, bad]
            rng.shuffle(opts)
            q = "Which sentence is grammatical?"
            out.append((decision_prompt(q, opts), LETTERS[opts.index(good)]))
    rng.shuffle(out)
    return out


def write_eval(path, held):
    """Held-out decisions in the format of `en decide --eval`."""
    rows = ["# question\tcontext\toptions\tright (held-out toy decisions)"]
    for user, answer in held:
        lines = user.split("\n")
        question = lines[0].removeprefix("Question: ")
        options = [line[3:] for line in lines[1:]]
        rows.append(f"{question}\t\t{'|'.join(options)}\t{LETTERS.index(answer)}")
    path.write_text("\n".join(rows) + "\n", encoding="utf-8")


def train_tokenizer(corpus, vocab):
    tok = Tokenizer(models.BPE())
    tok.pre_tokenizer = pre_tokenizers.ByteLevel(add_prefix_space=False)
    tok.decoder = decoders.ByteLevel()
    trainer = trainers.BpeTrainer(
        vocab_size=vocab,
        special_tokens=SPECIAL,
        initial_alphabet=pre_tokenizers.ByteLevel.alphabet(),
        show_progress=False,
    )
    tok.train_from_iterator(corpus, trainer)
    return tok


def gguf_permute(w, n_head):
    """Hugging Face to GGUF row order for rotary query and key weights."""
    return (
        w.reshape(n_head, 2, w.shape[0] // n_head // 2, *w.shape[1:])
        .swapaxes(1, 2)
        .reshape(w.shape)
    )


def init_params(cfg, key):
    """A Llama in the Hugging Face layout: linear weights are (out, in)."""
    h, ff, v = cfg["hidden_size"], cfg["intermediate_size"], cfg["vocab_size"]
    hd = h // cfg["num_attention_heads"]
    kv = cfg["num_key_value_heads"] * hd
    shapes = {"model.embed_tokens.weight": (v, h), "lm_head.weight": (v, h)}
    for i in range(cfg["num_hidden_layers"]):
        p = f"model.layers.{i}."
        shapes.update(
            {
                p + "self_attn.q_proj.weight": (h, h),
                p + "self_attn.k_proj.weight": (kv, h),
                p + "self_attn.v_proj.weight": (kv, h),
                p + "self_attn.o_proj.weight": (h, h),
                p + "mlp.gate_proj.weight": (ff, h),
                p + "mlp.up_proj.weight": (ff, h),
                p + "mlp.down_proj.weight": (h, ff),
            }
        )
    params = {}
    keys = jax.random.split(key, len(shapes))
    for k, (name, shape) in zip(keys, sorted(shapes.items())):
        params[name] = 0.02 * jax.random.normal(k, shape, jnp.float32)
    params["model.norm.weight"] = jnp.ones((h,))
    for i in range(cfg["num_hidden_layers"]):
        params[f"model.layers.{i}.input_layernorm.weight"] = jnp.ones((h,))
        params[f"model.layers.{i}.post_attention_layernorm.weight"] = jnp.ones((h,))
    return params


def forward(params, ids, cfg):
    """Logits for token ids of shape (batch, length)."""
    nh, nkv = cfg["num_attention_heads"], cfg["num_key_value_heads"]
    eps = cfg["rms_norm_eps"]
    b, t = ids.shape
    x = params["model.embed_tokens.weight"][ids]
    hd = x.shape[-1] // nh

    def norm(x, w):
        return x * jax.lax.rsqrt(jnp.mean(x * x, -1, keepdims=True) + eps) * w

    inv = 1.0 / (cfg["rope_theta"] ** (jnp.arange(0, hd, 2) / hd))
    ang = jnp.arange(t)[:, None] * inv[None, :]
    cos = jnp.cos(jnp.concatenate([ang, ang], -1))
    sin = jnp.sin(jnp.concatenate([ang, ang], -1))

    def rope(q):
        q1, q2 = q[..., : hd // 2], q[..., hd // 2 :]
        return q * cos + jnp.concatenate([-q2, q1], -1) * sin

    mask = jnp.tril(jnp.ones((t, t), bool))
    for i in range(cfg["num_hidden_layers"]):
        p = f"model.layers.{i}."
        y = norm(x, params[p + "input_layernorm.weight"])
        q = (y @ params[p + "self_attn.q_proj.weight"].T).reshape(b, t, nh, hd)
        k = (y @ params[p + "self_attn.k_proj.weight"].T).reshape(b, t, nkv, hd)
        v = (y @ params[p + "self_attn.v_proj.weight"].T).reshape(b, t, nkv, hd)
        q, k = rope(q.transpose(0, 2, 1, 3)), rope(k.transpose(0, 2, 1, 3))
        v = v.transpose(0, 2, 1, 3)
        k = jnp.repeat(k, nh // nkv, axis=1)
        v = jnp.repeat(v, nh // nkv, axis=1)
        att = (q @ k.transpose(0, 1, 3, 2)) / jnp.sqrt(hd)
        att = jax.nn.softmax(jnp.where(mask, att, -1e9), -1)
        o = (att @ v).transpose(0, 2, 1, 3).reshape(b, t, nh * hd)
        x = x + o @ params[p + "self_attn.o_proj.weight"].T
        y = norm(x, params[p + "post_attention_layernorm.weight"])
        g = jax.nn.silu(y @ params[p + "mlp.gate_proj.weight"].T)
        u = y @ params[p + "mlp.up_proj.weight"].T
        x = x + (g * u) @ params[p + "mlp.down_proj.weight"].T
    return norm(x, params["model.norm.weight"]) @ params["lm_head.weight"].T


def export_gguf(path, args, weights, tok):
    vocab = tok.get_vocab()
    tokens = [None] * len(vocab)
    for t, i in vocab.items():
        tokens[i] = t
    merges = json.loads(tok.to_str())["model"]["merges"]
    merges = [m if isinstance(m, str) else " ".join(m) for m in merges]
    types = [
        gguf.TokenType.CONTROL if t in SPECIAL else gguf.TokenType.NORMAL for t in tokens
    ]
    w = gguf.GGUFWriter(str(path), "llama")
    w.add_name("emdysi tiny test model")
    w.add_context_length(args['max_position_embeddings'])
    w.add_embedding_length(args['hidden_size'])
    w.add_block_count(args['num_hidden_layers'])
    w.add_feed_forward_length(args['intermediate_size'])
    w.add_head_count(args['num_attention_heads'])
    w.add_head_count_kv(args['num_key_value_heads'])
    w.add_rope_dimension_count(args['hidden_size'] // args['num_attention_heads'])
    w.add_layer_norm_rms_eps(args['rms_norm_eps'])
    w.add_rope_freq_base(args['rope_theta'])
    w.add_vocab_size(len(tokens))
    w.add_file_type(gguf.LlamaFileType.ALL_F32)
    w.add_tokenizer_model("gpt2")
    w.add_tokenizer_pre("gpt-2")
    w.add_token_list(tokens)
    w.add_token_types(types)
    w.add_token_merges(merges)
    w.add_bos_token_id(vocab["<|endoftext|>"])
    w.add_eos_token_id(vocab["<|im_end|>"])
    w.add_add_bos_token(False)
    w.add_chat_template(CHAT_TEMPLATE)
    names = {
        "model.embed_tokens.weight": "token_embd.weight",
        "model.norm.weight": "output_norm.weight",
        "lm_head.weight": "output.weight",
    }
    per_layer = {
        "input_layernorm.weight": "attn_norm.weight",
        "self_attn.q_proj.weight": "attn_q.weight",
        "self_attn.k_proj.weight": "attn_k.weight",
        "self_attn.v_proj.weight": "attn_v.weight",
        "self_attn.o_proj.weight": "attn_output.weight",
        "post_attention_layernorm.weight": "ffn_norm.weight",
        "mlp.gate_proj.weight": "ffn_gate.weight",
        "mlp.up_proj.weight": "ffn_up.weight",
        "mlp.down_proj.weight": "ffn_down.weight",
    }
    for i in range(args['num_hidden_layers']):
        for k, v in per_layer.items():
            names[f"model.layers.{i}.{k}"] = f"blk.{i}.{v}"
    for k, v in weights.items():
        a = np.asarray(v, dtype=np.float32)
        if k.endswith("q_proj.weight"):
            a = gguf_permute(a, args['num_attention_heads'])
        elif k.endswith("k_proj.weight"):
            a = gguf_permute(a, args['num_key_value_heads'])
        w.add_tensor(names[k], a)
    w.write_header_to_file()
    w.write_kv_data_to_file()
    w.write_tensors_to_file()
    w.close()


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--out", default=str(pathlib.Path.home() / ".cache/emdysi/tiny-model"))
    ap.add_argument("--steps", type=int, default=1500)
    ap.add_argument("--vocab", type=int, default=4096)
    ap.add_argument("--hidden", type=int, default=256)
    ap.add_argument("--layers", type=int, default=4)
    ap.add_argument("--seq", type=int, default=256)
    ap.add_argument("--batch", type=int, default=16)
    ap.add_argument("--seed", type=int, default=7)
    a = ap.parse_args()
    rng = random.Random(a.seed)
    out = pathlib.Path(a.out)
    (out / "mlx").mkdir(parents=True, exist_ok=True)

    prose = texts()
    dec = decisions(rng)
    held = dec[: len(dec) // 10]
    dec = dec[len(dec) // 10 :]
    write_eval(out / "eval.tsv", held)
    chats = [chat(u, ans) for u, ans in dec]
    tok = train_tokenizer(prose + chats, a.vocab)
    tok.save(str(out / "mlx" / "tokenizer.json"))
    eot = tok.token_to_id("<|endoftext|>")

    # Interleave prose and decision chats into one token stream.
    docs = [(t, False) for t in prose] + [(c, True) for c in chats]
    rng.shuffle(docs)
    stream = []
    for d, _ in docs:
        stream.extend(tok.encode(d).ids + [eot])
    print(f"{len(prose)} texts, {len(chats)} decisions, {len(stream)} tokens")

    cfg = {
        "hidden_size": a.hidden,
        "num_hidden_layers": a.layers,
        "intermediate_size": a.hidden * 3,
        "num_attention_heads": 4,
        "num_key_value_heads": 2,
        "rms_norm_eps": 1e-5,
        "vocab_size": tok.get_vocab_size(),
        "rope_theta": 10000.0,
        "max_position_embeddings": 1024,
    }
    params = init_params(cfg, jax.random.PRNGKey(a.seed))
    print(f"{sum(v.size for v in params.values()) / 1e6:.1f}M parameters")

    def loss_fn(params, x, y):
        logits = forward(params, x, cfg)
        return optax.softmax_cross_entropy_with_integer_labels(logits, y).mean()

    warmup = min(50, a.steps // 2)
    sched = optax.warmup_cosine_decay_schedule(0.0, 2e-3, warmup, max(a.steps, warmup + 1))
    opt = optax.adamw(sched, weight_decay=0.01)
    state = opt.init(params)

    @jax.jit
    def step(params, state, x, y):
        loss, grads = jax.value_and_grad(loss_fn)(params, x, y)
        updates, state = opt.update(grads, state, params)
        return optax.apply_updates(params, updates), state, loss

    data = np.array(stream, dtype=np.int32)
    n = data.size - a.seq - 1
    t0 = time.time()
    for i in range(a.steps):
        starts = [rng.randrange(n) for _ in range(a.batch)]
        x = np.stack([data[s : s + a.seq] for s in starts])
        y = np.stack([data[s + 1 : s + a.seq + 1] for s in starts])
        params, state, loss = step(params, state, x, y)
        if i % 100 == 0 or i == a.steps - 1:
            print(f"step {i} loss {float(loss):.3f} ({time.time() - t0:.0f} s)", flush=True)

    # Held-out accuracy on the toy decisions: the more probable letter.
    fwd = jax.jit(lambda p, ids: forward(p, ids, cfg))
    a_id, b_id = tok.token_to_id("A"), tok.token_to_id("B")
    right = 0
    for u, ans in held:
        prompt = chat(u, "").rsplit("<|im_end|>", 1)[0]
        logits = fwd(params, np.array([tok.encode(prompt).ids]))[0, -1]
        right += ("A" if logits[a_id] > logits[b_id] else "B") == ans
    print(f"held-out decisions: {right}/{len(held)} right")

    weights = {k: np.asarray(v, dtype=np.float32) for k, v in params.items()}
    save_file(weights, str(out / "mlx" / "model.safetensors"), metadata={"format": "mlx"})
    config = {
        "model_type": "llama",
        "architectures": ["LlamaForCausalLM"],
        **cfg,
        "tie_word_embeddings": False,
        "bos_token_id": eot,
        "eos_token_id": tok.token_to_id("<|im_end|>"),
    }
    (out / "mlx" / "config.json").write_text(json.dumps(config, indent=2))
    (out / "mlx" / "tokenizer_config.json").write_text(
        json.dumps(
            {
                "tokenizer_class": "PreTrainedTokenizerFast",
                "bos_token": "<|endoftext|>",
                "eos_token": "<|im_end|>",
                "pad_token": "<|endoftext|>",
                "chat_template": CHAT_TEMPLATE,
                "model_max_length": cfg["max_position_embeddings"],
            },
            indent=2,
        )
    )
    export_gguf(out / "tiny.gguf", cfg, weights, tok)
    print(f"wrote {out / 'mlx'} and {out / 'tiny.gguf'}")


if __name__ == "__main__":
    main()
