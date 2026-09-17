# Credential expiry CLI

The Docker target accepts an integer expiry argument and LOGICAL_NOW environment
value. It implements the existing P8 credential contract. The known mutant always
accepts credentials. Build with a preloaded digest-pinned Node image; the runner
copies the selected candidate to target.js and builds without network or pulls.
The P8 suite constructor runs in a separate disposable controller lane. This is
public operational-separation testing, not independent hidden-holdout evidence.
