const BITGPU_VERSION = "0.19.1";
const MODEL_REVISION = "78f2c2bacd0904ffaba24b4873ed975e5818354a";
const TOKENIZER_REVISION = "584f02f4ca20c85ff8b5d38d2323cef2d24decd0";
const MODEL_FILES =
  `https://cdn.jsdelivr.net/gh/stfurkan/bitgpu@v${BITGPU_VERSION}/models/bonsai-4b-gguf`;
const TOKENIZER_FILES =
  `https://huggingface.co/onnx-community/Bonsai-4B-ONNX/resolve/${TOKENIZER_REVISION}`;

export const ASK_MODEL = /** @type {const} */ ({
  name: "Bonsai 4B",
  size: "~580 MB",
  maxSeqLen: 4_096,
  runtimeUrl: `https://esm.sh/bitgpu@${BITGPU_VERSION}`,
  chatUrl: `https://esm.sh/bitgpu@${BITGPU_VERSION}/chat`,
  manifestUrl: `${MODEL_FILES}/manifest.json`,
  auxUrl: `${MODEL_FILES}/Bonsai-4B-Q1_0.aux.bin`,
  dataUrl:
    `https://huggingface.co/prism-ml/Bonsai-4B-gguf/resolve/${MODEL_REVISION}/Bonsai-4B-Q1_0.gguf`,
  tokenizerJsonUrl: `${TOKENIZER_FILES}/tokenizer.json`,
  tokenizerConfigUrl: `${TOKENIZER_FILES}/tokenizer_config.json`,
  cacheName: "bitview-ask-bonsai-4b-v1",
});
