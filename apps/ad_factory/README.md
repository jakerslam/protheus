# Ad Factory (Default App Scaffold)

Purpose: a default top-level app for generating marketing ad scripts/videos at scale.

Boundary:
- This app sits in `/apps` and is not part of `client/`.
- It must call client/core capabilities through conduit-governed interfaces.

Planned lanes (V6-COCKPIT-016):
- Script generation
- UGC-style video generation
- Batch ad factory mode
- Cost/receipt tracking
