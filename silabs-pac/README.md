# Modifying the PAC
- Add new rules to transform.yaml
- run ```chiptool generate --svd .\svd\EFR32MG24B220F1536IM48.svd.patched --transform transform.yaml```
- run ```rustfmt lib.rs && rm src/lib.rs && mv lib.rs src/lib.rs```