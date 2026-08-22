# Audio8 TTS for Android

OLED-black Tron HUD for **[Audio8 TTS](https://github.com/Audio8-AI/Audio8_TTS)**: DualAR 0.6B / 0.1B text-to-speech, zero-shot cloning, ONNX INT4 serving, SGLang Omni, batch JSONL, codec codes, and every SFT hyperparameter the upstream CLIs expose.

The UI is the [RustVision UI kit](https://github.com/Arrsenio/rustvision-ui-kit) (egui 0.31, cargo-apk2, sticky immersive). **Everything that can live in Rust does.** Java is only the NativeActivity subclass, immersive insets, and `ACTION_GET_CONTENT` (Android cannot deliver picker results without an Activity).

## What you can drive from the phone

| Tab | Upstream surface |
| --- | --- |
| **SYNTH** | `audio8_tts_infer.py` + ONNX `/api/tts`: text, voice, language (11 Preview langs), temperature, top-p, top-k, seed, greedy/argmax, max-new-tokens, retry-max-new-tokens, stream, chunk/context/guard frames, response format WAV/PCM/CODES, save-codes, overwrite, model id |
| **CLONE** | Zero-shot pair `reference_audio` + `reference_text`, `/api/voices/register` (name, overwrite, 0.5–30 s / ≤50 MiB), SGLang `references[]`, voice list |
| **BATCH** | JSONL manifests (`id`, `text`, `voice`, `reference_audio`, `reference_text`) with the same grouping rule as the Python CLI |
| **CODES** | `[10, T]` codebook inspector + `.npy` (uint16) — the DualAR acoustic tokens |
| **ENGINE** | ONNX HTTP (`:8024`), SGLang Omni (`:8010`), local DualAR; URL, model dir, voices dir, registration dir, INT4/FP16, threads, device, dtype, timeout, `/api/health`, `/api/system`, `/api/runtime/reload`, `/api/tts/cancel` |
| **SFT** | Every `audio8_tts_sft.sh` / `ModelArguments` / `DataArguments` / `Audio8TTSTrainingArguments` flag (freeze slow/fast AR, loss weights, DeepSpeed-side batch/LR/epochs, nnodes). Exports the shell command + JSON. Training itself stays on a GPU box. |
| **ARCHIVE** | WAV + codes + the exact params used |

Rust ports (not wrappers):

- `audio8::text` — `clean_text`, CJK newline folding, `<\|speaker:N\|>`, sample ids  
- `audio8::sample` — nucleus + Gumbel-max, semantic anti-repeat, greedy  
- `audio8::dualar` — Slow AR → Fast AR 10-codebook loop, prompt packing, EOS  
- `audio8::npy` / `wav` / `voice` — codec arrays, 44.1 kHz PCM, voice profiles  
- `audio8::http` — full ONNX + OpenAI-compatible + NDJSON/SSE streaming client  

Local on-device ONNX: implement `DualArBackend` with `ort` against `slow_ar_int4.onnx`, `fast_ar_int4.onnx`, `codec_decoder_fp16.onnx`. The generate loop is already Rust.

## Build / install

```bash
export ANDROID_HOME="$HOME/android-sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.2.12479018"
export PATH="$ANDROID_HOME/platform-tools:$PATH"

rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-apk2

cd audio8-tts-android
cargo test
cargo apk2 build --release
adb install -r target/aarch64-linux-android/release/apk/audio8-tts.apk
```

Point **ENGINE** at a machine running Audio8:

```bash
# ONNX INT4 CPU service
cd Audio8_TTS/onnx_runtime && bash start_server.sh   # :8024

# or SGLang Omni
HOST=0.0.0.0 PORT=8010 ./sglang_omni/scripts/run_server.sh
```

On the emulator, `10.0.2.2` is the host. On a USB device:

```bash
adb reverse tcp:8024 tcp:8024
# then BASE URL http://127.0.0.1:8024
```

## License

Apache-2.0, same as Audio8 TTS. Tron widgets follow the RustVision UI kit.
