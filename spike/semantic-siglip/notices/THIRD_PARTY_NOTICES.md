# Third-party notices

## Google SigLIP 2

- Model: `google/siglip2-base-patch16-512`
- Pinned revision: `a89f5c5093f902bf39d3cd4d81d2c09867f0724b`
- Official card: https://huggingface.co/google/siglip2-base-patch16-512
- Official API: https://huggingface.co/api/models/google/siglip2-base-patch16-512
- Both the card and API `cardData.license` declare `apache-2.0`.
- The pinned card is retained as `Google-SigLIP2-README.md`. The repository file list has no upstream `NOTICE` file and no official ONNX file.
- Canonical Apache license: https://www.apache.org/licenses/LICENSE-2.0.txt, retained as `Apache-2.0.txt`.
- The model card identifies the paper “SigLIP 2: Multilingual Vision-Language Encoders with Improved Semantic Understanding, Localization, and Dense Features”, arXiv:2502.14786, and lists image-text retrieval as an intended use.

Official interface evidence from the pinned files and Transformers 4.56.2 loader:

- `config.json` has `model_type: siglip`; `AutoModel` therefore loads `SiglipModel`, despite the repository's SigLIP 2 name.
- Text: 64 positions, hidden size 768, projection size 768. The final vector is final LayerNorm -> last token -> Linear(768,768), not an unprojected `last_hidden_state`.
- Vision: 512×512, patch 16. The final vector is the 768-dimensional `SiglipMultiheadAttentionPoolingHead` output.
- `preprocessor_config.json`: direct resize to height=512,width=512, `resample: 2` (bilinear), rescale 1/255, mean/std 0.5. No bicubic shortest-edge/center-crop rule is inferred.
- Tokenizer: fast Gemma tokenizer, no BOS, EOS enabled, pad `<pad>` ID 0, fixed to 64 for this model. The official call returns `input_ids` without `attention_mask`; the exported text graph accordingly takes only `input_ids`.

## Official smoke image

`../assets/CLIP.png` and its notices are copied from the read-only multilingual spike. Original source revision: `d05afc436d78f1c48dc0dbf8e5980a9d471f35f6`.

- Image: https://raw.githubusercontent.com/openai/CLIP/d05afc436d78f1c48dc0dbf8e5980a9d471f35f6/CLIP.png
- MIT license: https://github.com/openai/CLIP/blob/d05afc436d78f1c48dc0dbf8e5980a9d471f35f6/LICENSE
- The MIT text and upstream README are retained as `OpenAI-CLIP-LICENSE.txt` and `OpenAI-CLIP-README.md`.

## Private runtime

`../runtime/licenses/` contains the original notices for `Microsoft.ML.OnnxRuntime.DirectML` 1.24.4 and `Microsoft.AI.DirectML` 1.15.4. Package URLs, DLL hashes and read-only copy provenance are in `../outputs/runtime-manifest.json`.

## Quality dataset

The separately maintained `spike/semantic-eval/manifest/quality-manifest.json` identifies BridgeData V2 as CC BY 4.0 and supplies attribution to “BridgeData V2, Homer Walke et al., RAIL Berkeley”. Its images and query manifests are read in place and are not relicensed by this spike.
