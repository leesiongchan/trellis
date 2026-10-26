# Branch portability patch

Upstream: fastembed 7.5.0, https://crates.io/crates/fastembed/7.5.0
Crate archive SHA-256: `b65a7054d36362ab4c234d7b4b58f3f453e9ec73643283223f0b170554481906`. Original Apache-2.0 LICENSE is retained.

Only `src/text_embedding/init.rs` changes: add an optional `auto_device_policy` to user-defined initialization and apply ort's safe `SessionBuilder::with_auto_device` API. Default behavior remains unchanged. Branch explicitly chooses `PreferCPU`.

ort 2.0.0-rc.15 automatically selects CoreML on macOS; the unmodified fastembed API cannot override that policy. The offline native gate reproduced CoreML attempting to compile a model cache and failing. This patch preserves the pinned embedding model/provider, CPU preprocessing parity, two inference threads and a single native process. Remove the vendor override when upstream exposes an equivalent option, after repeating the offline native and ranking checks.
