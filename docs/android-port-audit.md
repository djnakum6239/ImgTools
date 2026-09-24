# ImageForge -> ImgTools Android Port Audit

Audit baseline: ImageForge main at 39607a230c7f9b4f4cc1ddf4c0cb8276bf77a7a5 (2026-09-23).

This audit is the implementation contract for ImgTools. The Android project will not be treated as a collection of UI buttons around an APK build. Each upstream feature is traced from UI -> worker/session -> core implementation -> WASM/reference dependency -> tests/reference material before it is ported.

## Critical finding

ImageForge is layered: UI -> application state/worker -> patch engine -> compatibility -> artifact registry -> patch providers -> image/package/partition/logo/animation engines -> worker workspace -> WASM helpers.

The most important portability finding is that crates/imageforge-wasm is NOT the main ImageForge engine. It contains binary helpers: CRC32, LZ4 block coding, XZ and BZip2 helpers. The majority of image parsing, package handling, partition parsing, patch planning and provider logic lives in src/core/. Therefore copying the WASM crate into Android is only a codec/native-helper port, not an ImageForge port.

## Feature audit

| Feature | Upstream implementation | Android implementation target |
|---|---|---|
| Inspect/detect | workspace/detect.ts, image parsers, image/report.ts | Rust detector + report API |
| Boot image parse/repack/verify | image/bootimage/* | Rust parser/writer/verifier |
| Compression | image/compression.ts, gzip/lz4/xz/bzip2/zlib | Rust codec layer with reference-compatible behavior |
| Ramdisk/CPIO | image/ramdisk.ts, cpio.ts | Rust CPIO + ramdisk layer |
| OTA/ZIP/package | package/* | SAF range source + Rust package reader |
| OTA payload streaming | package/payload.ts + payload-stream.ts | Rust CrAU/protobuf/operation reader with streaming |
| Sparse | partition/sparse.ts + sparse-write.ts | Rust AOSP-compatible reader/writer |
| Super/liblp | partition/lp.ts + super-write.ts | Rust liblp-compatible parser/writer |
| EROFS | partition/erofs.ts + erofs-z.ts | Rust read-only filesystem browser |
| ext4 | partition/ext4.ts | Rust read-only filesystem browser |
| Logo/splash | logo/* | Rust parser/preview/packer |
| Boot animation | animation/* | Rust ZIP/desc parser/packer |
| Diff | diff/* | Rust range/streaming diff |
| Workspace | workspace/* | Kotlin metadata + Rust-owned byte artifacts |
| Patch engine | patch/engine/* | Rust planner/resolver/executor/verifier |
| APatch | patch/providers/apatch-provider.ts + kptools WASI | native Rust/provider boundary |
| KernelSU family | patch/providers/kernelsu-provider.ts | Rust ramdisk/module implementation |
| Magisk family | patch/providers/magisk-provider.ts | Rust ramdisk implementation |
| Artifact verification | artifacts/* | local digest-pinned registry |
| Progress/cancellation | workers/session.ts + protocol.ts | Kotlin coroutine/job + Rust cancellation |
| Persistence | workers/persistence.ts | app-private persistence + persisted SAF permissions |
| Offline operation | local worker + bundled WASM/artifacts | no network dependency/permission |
| i18n/settings/theme | i18n/* + stores/* + SettingsPage | Compose resources/preferences |

## File-by-file inventory

### Application/UI
- src/app/App.tsx — application root.
- src/app/AppShell.tsx — shell/layout.
- src/app/Header.tsx — header/workflow navigation.
- src/app/router.tsx — route graph and legacy redirects.
- src/app/route-fallback.tsx — lazy-route fallback.
- src/app/tools.ts — authoritative tool registry and artifact-kind contracts.
- src/app/workflow.ts — patch workflow step mapping.

### Image engine
- src/core/image/types.ts — image/header/section data contracts.
- src/core/image/index.ts — image-engine exports.
- src/core/image/architecture.ts — architecture detection.
- src/core/image/binary.ts — binary reader/writer primitives.
- src/core/image/canonical.ts — deterministic canonicalization.
- src/core/image/report.ts — inspection report generation.
- src/core/image/compression.ts — compression detection/dispatch.
- src/core/image/gzip.ts — gzip handling.
- src/core/image/lz4.ts — LZ4 legacy/frame parsing and encoding.
- src/core/image/xz.ts — XZ decode/encode and dictionary declaration.
- src/core/image/bzip2.ts — BZip2 reference/native bridge.
- src/core/image/zlib.ts — zlib handling.
- src/core/image/cpio.ts — CPIO newc/crc parsing and writing.
- src/core/image/ramdisk.ts — ramdisk extraction/repack.
- src/core/image/elf.ts — ELF inspection/module validation.
- src/core/image/kernelrelease.ts — kernel release/KMI extraction.
- src/core/image/zip-write.ts — deterministic stored ZIP writer.
- src/core/image/bootimage/constants.ts — Android boot constants.
- src/core/image/bootimage/header.ts — boot/vendor_boot headers.
- src/core/image/bootimage/parser.ts — complete image parsing.
- src/core/image/bootimage/repacker.ts — boot image rebuild.
- src/core/image/bootimage/vendor-repacker.ts — vendor_boot rebuild.
- src/core/image/bootimage/verifier.ts — structural/AVB verification.

### Package engine
- src/core/package/source.ts — range-based ByteSource abstraction.
- src/core/package/zip.ts — ZIP/ZIP64 reader.
- src/core/package/protobuf.ts — minimal protobuf wire reader.
- src/core/package/payload.ts — Android CrAU payload parser/materializer.
- src/core/package/payload-stream.ts — operation-by-operation streaming extraction.
- src/core/package/payload-source.ts — payload partition source abstraction.
- src/core/package/index.ts — package exports.

### Partition engine
- src/core/partition/sparse.ts — AOSP sparse reader.
- src/core/partition/sparse-write.ts — AOSP sparse writer.
- src/core/partition/lp.ts — liblp metadata parser.
- src/core/partition/super-write.ts — lpmake-compatible writer.
- src/core/partition/erofs.ts — EROFS metadata/path reader.
- src/core/partition/erofs-z.ts — EROFS compressed-file/LZ4 support.
- src/core/partition/ext4.ts — ext4 superblock/inode/extent/directory/file reader.
- src/core/partition/index.ts — partition exports.

### Patch engine/providers
- src/core/patch/types.ts — plans, options, attachments, progress and provider contracts.
- src/core/patch/engine/index.ts — engine composition.
- src/core/patch/engine/planner.ts — plan construction.
- src/core/patch/engine/resolver.ts — provider/artifact resolution.
- src/core/patch/engine/executor.ts — execution/cancellation/progress.
- src/core/patch/engine/verifier.ts — post-patch verification.
- src/core/patch/providers/registry.ts — lazy provider registry.
- src/core/patch/providers/descriptors.ts — provider declarations.
- src/core/patch/providers/apatch-provider.ts — APatch/KernelPatch.
- src/core/patch/providers/apatch-config.ts — APatch options/flavours.
- src/core/patch/providers/kpm-info.ts — KernelPatch module validation.
- src/core/patch/providers/kernelsu-provider.ts — KernelSU family.
- src/core/patch/providers/kernelsu-config.ts — KernelSU configuration.
- src/core/patch/providers/kernelsu-module-config.ts — module metadata/config.
- src/core/patch/providers/magisk-provider.ts — Magisk family.
- src/core/patch/providers/magisk-config.ts — Magisk configuration.
- src/core/patch/providers/ramdisk-support.ts — shared ramdisk operations.
- src/core/patch/providers/output-options.ts — output/signature/size options.
- src/core/patch/providers/manager-apps.ts — manager identity metadata.
- src/core/patch/providers/mock-provider.ts and mock-config.ts — test-only pipeline provider.

### Workspace/artifacts/compatibility/diff
- src/core/compat/engine.ts and types.ts — provider compatibility.
- src/core/artifacts/catalog.ts, registry.ts, types.ts — releases, digests and payload loading.
- src/core/workspace/detect.ts — magic-based classification.
- src/core/workspace/kinds.ts — artifact/tool vocabulary.
- src/core/workspace/graph.ts — artifact lineage.
- src/core/workspace/matching.ts — tool applicability.
- src/core/workspace/index.ts — workspace exports.
- src/core/diff/diff.ts and index.ts — byte and boot-section diffing.
- src/core/errors.ts — typed engine errors.
- src/core/hash.ts — hashing.

### Logo/animation
- src/core/logo/formats.ts — format definitions.
- src/core/logo/bmp.ts — BMP decode/encode.
- src/core/logo/splash.ts — OPPO/Realme/OnePlus splash container.
- src/core/logo/mtk.ts — MediaTek logo container/pixel layouts.
- src/core/logo/adapt.ts — image fitting.
- src/core/logo/index.ts — logo exports.
- src/core/animation/desc.ts — AOSP/vendor desc.txt parser/editor.
- src/core/animation/pack.ts — bootanimation ZIP handling.
- src/core/animation/index.ts — animation exports.

### Worker/WASM boundary
- src/workers/protocol.ts — complete worker API/data contract.
- src/workers/session.ts — source/artifact/workspace lifecycle and tool operations.
- src/workers/client.ts — main-thread RPC client.
- src/workers/persistence.ts — workspace persistence.
- src/workers/patch.worker.ts — worker entrypoint.
- src/wasm/abi.ts — codec ABI.
- src/wasm/loader.ts — WASM loading/status.
- src/wasm/assets.ts — pinned WASM artifact registry.
- src/wasm/fallback.ts — TypeScript codec fallbacks.
- src/wasm/lz4-codec.ts — reference liblz4 loader.
- src/wasm/bzip2-codec.ts — reference bzip2 loader.
- src/wasm/wasi-runner.ts — WASI execution for bundled kptools.

## Verification requirements

Upstream has 20 fixture modules, 50 unit tests, 29 integration tests, 5 worker tests and 2 WASM tests. The Android port must preserve the same verification intent: damaged-input rejection, reference-tool comparisons, real device/OTA material where available, provider output checks, workspace persistence and UI behavior.

Important reference material documented upstream includes an 8.2 GB OTA, AOSP lpmake/img2simg, real boot/vendor_boot/init_boot dumps, EROFS/ext4 sha256 digests, real bootanimation/logo samples, KernelPatch kptools and real provider modules.

## Current ImgTools gap

The current Android repository is a skeleton. It has a Compose shell, SAF picker, JNI version/CRC32 calls and the copied native codec helper. It does not yet contain the ImageForge image/package/partition engines, workspace/session model, patch engine, APatch/KernelSU/Magisk providers, OTA streaming, filesystem browsers/writers, logo/animation processing, golden/reference tests, SAF random-access infrastructure or background/cancellation orchestration.

## Porting rules

1. Do not treat the WASM helper crate as the full engine.
2. Preserve range access: an OTA can be about 8.2 GB and must never be read wholesale.
3. Preserve streaming artifacts for multi-gigabyte partitions.
4. Keep provider implementations separate because their payloads, signatures and verification differ.
5. Keep device flashing out of scope; the app produces files.
6. Keep processing offline and digest-verify bundled artifacts.
7. SAF is the Android equivalent of browser File/Blob handles and needs persisted URI permissions.
8. Build success is only one gate; feature parity and golden/reference tests are separate gates.

Every implementation increment will inspect the relevant upstream source and tests first, implement the Android/Rust equivalent, add focused tests, build all native ABIs, build the APK, and inspect every step of the resulting Actions run including every failure before proceeding.