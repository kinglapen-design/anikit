# Third-party notices

## Multilingual text tower

- Model: `sentence-transformers/clip-ViT-B-32-multilingual-v1`
- Pinned Hugging Face revision: `58edf8cada9e398793dca955574a48cbb7f18be2`
- Official model card: https://huggingface.co/sentence-transformers/clip-ViT-B-32-multilingual-v1
- Official API metadata: https://huggingface.co/api/models/sentence-transformers/clip-ViT-B-32-multilingual-v1
- The model card and API `cardData.license` declare `apache-2.0`.
- The card states that the teacher was the original `clip-ViT-B-32`, that multilingual DistilBERT was distilled into the teacher vector space, and that the CLIP image encoder is unchanged. Its architecture is Transformer -> mean Pooling -> Dense(768, 512).
- Canonical license text: https://www.apache.org/licenses/LICENSE-2.0.txt, retained as `Apache-2.0.txt`.
- The pinned model card is retained as `SentenceTransformers-clip-multilingual-README.md`. No upstream `NOTICE` file is listed by the model repository API.

## Original OpenAI image tower and official sample

- Converted checkpoint: `openai/clip-vit-base-patch32`
- Pinned Hugging Face revision: `3d74acf9a28c67741b2f4f2ea7635f0aaf6f0268`
- Mirror card: https://huggingface.co/openai/clip-vit-base-patch32
- Mirror API metadata: https://huggingface.co/api/models/openai/clip-vit-base-patch32
- The mirror does **not** declare a license tag; its metadata is not the basis for the MIT conclusion.
- Upstream OpenAI CLIP revision: `d05afc436d78f1c48dc0dbf8e5980a9d471f35f6`.
- Official MIT license: https://github.com/openai/CLIP/blob/d05afc436d78f1c48dc0dbf8e5980a9d471f35f6/LICENSE (copyright (c) 2021 OpenAI), retained as `OpenAI-CLIP-LICENSE.txt`.
- At that same revision, the official loader directly publishes the ViT-B/32 weight URL: https://github.com/openai/CLIP/blob/d05afc436d78f1c48dc0dbf8e5980a9d471f35f6/clip/clip.py#L36. The loader source, README and model card are retained here. This is the official evidence used for the OpenAI CLIP code/official-weight MIT path.
- The pinned converted-mirror card is retained as `OpenAI-clip-hf-README.md`. A redistribution review should record that this is a converted checkpoint and that the mirror itself omits license metadata.
- Official sample `CLIP.png` comes from the same MIT repository revision and is retained in `../assets/CLIP.png`.
- The OpenAI model card's research-use and deployment cautions are retained as usage guidance; they are not silently rewritten as copyright terms. In-domain evaluation remains necessary.

## Private Windows runtime

The ORT/DirectML license texts and third-party notices are in `../runtime/licenses/`:

- `Microsoft.ML.OnnxRuntime.DirectML` 1.24.4: `OnnxRuntime-LICENSE.txt`, `OnnxRuntime-ThirdPartyNotices.txt`.
- `Microsoft.AI.DirectML` 1.15.4: `DirectML-LICENSE.txt`, `DirectML-LICENSE-CODE.txt`, `DirectML-ThirdPartyNotices.txt`.

Package URLs and SHA256 values are in `../outputs/runtime-manifest.json`.

## Quality dataset

The separately maintained `spike/semantic-eval/manifest/quality-manifest.json` identifies BridgeData V2 as CC BY 4.0 and supplies attribution to “BridgeData V2, Homer Walke et al., RAIL Berkeley”. Its images are read in place and are not copied into this spike or covered by this spike's model licenses.
