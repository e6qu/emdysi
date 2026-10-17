# Abstract

In recent years, large-scale distributed systems have become increasingly central to modern computing infrastructure. However, ensuring their reliability remains a significant and ongoing challenge. In this paper, we present a novel framework for detecting anomalies in distributed traces that leverages graph-based representations to capture the intricate relationships between services.

Our approach addresses several key limitations of existing methods. First, it scales efficiently to systems with thousands of services. Second, it requires minimal manual configuration. Third, it provides interpretable explanations that help engineers quickly identify root causes.

We evaluate our framework on three real-world datasets and demonstrate that it consistently outperforms state-of-the-art baselines, achieving an average improvement of 12% in detection accuracy. Furthermore, our results highlight the importance of modeling service dependencies explicitly.

These findings have important implications for both researchers and practitioners. By shedding light on the structure of failures in distributed systems, our work paves the way for more robust and reliable infrastructure. We believe this represents a meaningful step toward the broader goal of self-healing systems.
