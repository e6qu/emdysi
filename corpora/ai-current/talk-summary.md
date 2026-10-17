# Key Takeaways from "Scaling Search Without Losing Your Mind"

Last week's session at the Infrastructure Summit was easily one of the most insightful talks of the conference. The speaker, a staff engineer who has spent the last five years scaling search at a fast-growing marketplace, shared hard-won lessons that every engineering team can learn from. Here are the key takeaways.

## 1. Relevance Is a Product Problem, Not Just an Engineering One

One of the most striking points was that search quality can't be solved by engineers alone. Relevance depends on understanding what users actually mean — and that requires close collaboration between engineering, product, and data science. As the speaker put it: "The best ranking model in the world won't help you if you're optimizing for the wrong thing."

## 2. Measure What Matters

The team learned the hard way that vanity metrics can be misleading. Click-through rate looked great, but it didn't reflect whether users found what they needed. By shifting to task-completion metrics, they uncovered issues that had been hiding in plain sight.

## 3. Embrace Incremental Change

Rather than attempting a big-bang rewrite, the team migrated to their new architecture one query type at a time. This allowed them to validate each step, roll back quickly when needed, and maintain trust with stakeholders throughout the process.

## 4. Invest in Tooling Early

Perhaps the most practical takeaway: invest in internal tooling before you need it. A simple side-by-side comparison tool for search results ended up being one of the team's most valuable assets, enabling faster iteration and more confident decisions.

## Final Thoughts

Overall, the talk was a powerful reminder that scaling search is as much about people and process as it is about technology. Whether you're just starting your search journey or managing a mature system, these lessons offer a valuable roadmap for building search experiences that truly serve your users.
