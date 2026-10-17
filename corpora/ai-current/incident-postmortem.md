# Postmortem: API Degradation on March 3

## Summary

On March 3, our API experienced a period of elevated latency and intermittent errors lasting approximately 47 minutes. We understand how critical our platform is to your operations, and we sincerely apologize for the disruption.

## Root Cause

The incident was triggered by a configuration change that inadvertently reduced the connection pool size for our primary database. Under peak traffic, this led to a cascading effect, as requests queued up and timed out.

## Impact

During the incident window, approximately 18% of API requests failed or experienced significant delays. No data was lost.

## What We're Doing to Prevent This

We take incidents like this seriously, and we're committed to learning from them. Moving forward, we will:

1. Introduce automated validation for all configuration changes.
2. Enhance our monitoring to detect connection pool saturation earlier.
3. Strengthen our deployment process with staged rollouts.

## Conclusion

Reliability is at the core of everything we do. We're grateful for your patience and trust, and we remain committed to delivering the stable, high-performing service you deserve.
