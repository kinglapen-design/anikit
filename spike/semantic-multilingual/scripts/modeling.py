from __future__ import annotations

import json
from pathlib import Path

from safetensors import safe_open
import torch
from torch import nn
from torch.nn import functional as F
from transformers import AutoModel, CLIPModel

EMBEDDING_DIM = 512
CONTEXT_LENGTH = 128
IMAGE_SIZE = 224


class TextTower(nn.Module):
    def __init__(self, transformer: nn.Module, dense: nn.Linear) -> None:
        super().__init__()
        self.transformer = transformer
        self.dense = dense

    def forward(
        self, input_ids: torch.Tensor, attention_mask: torch.Tensor
    ) -> tuple[torch.Tensor, torch.Tensor]:
        hidden = self.transformer(
            input_ids=input_ids, attention_mask=attention_mask, return_dict=True
        ).last_hidden_state
        mask = attention_mask.unsqueeze(-1).to(hidden.dtype)
        pooled = (hidden * mask).sum(dim=1) / mask.sum(dim=1).clamp(min=1e-9)
        raw = self.dense(pooled)
        return raw, F.normalize(raw, p=2, dim=-1)


class VisionTower(nn.Module):
    def __init__(self, model: CLIPModel) -> None:
        super().__init__()
        self.vision_model = model.vision_model
        self.visual_projection = model.visual_projection

    def forward(self, pixel_values: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        pooled = self.vision_model(pixel_values=pixel_values).pooler_output
        raw = self.visual_projection(pooled)
        return raw, F.normalize(raw, p=2, dim=-1)


def load_text_tower(weights: Path) -> TextTower:
    modules = json.loads((weights / "modules.json").read_text(encoding="utf-8"))
    module_types = [item["type"] for item in sorted(modules, key=lambda item: item["idx"])]
    if module_types != [
        "sentence_transformers.models.Transformer",
        "sentence_transformers.models.Pooling",
        "sentence_transformers.models.Dense",
    ]:
        raise RuntimeError(f"Unexpected text module chain: {module_types}")
    pooling = json.loads((weights / "1_Pooling/config.json").read_text(encoding="utf-8"))
    if not pooling["pooling_mode_mean_tokens"] or any(
        pooling[name]
        for name in (
            "pooling_mode_cls_token",
            "pooling_mode_max_tokens",
            "pooling_mode_mean_sqrt_len_tokens",
        )
    ):
        raise RuntimeError(f"Expected mean-only pooling, got {pooling}")
    dense_config = json.loads((weights / "2_Dense/config.json").read_text(encoding="utf-8"))
    if dense_config != {
        "in_features": 768,
        "out_features": EMBEDDING_DIM,
        "bias": False,
        "activation_function": "torch.nn.modules.linear.Identity",
    }:
        raise RuntimeError(f"Unexpected dense config: {dense_config}")

    transformer = AutoModel.from_pretrained(weights, local_files_only=True)
    dense = nn.Linear(768, EMBEDDING_DIM, bias=False)
    with safe_open(weights / "2_Dense/model.safetensors", framework="pt") as source:
        if source.keys() != ["linear.weight"]:
            raise RuntimeError(f"Unexpected dense tensors: {source.keys()}")
        dense.weight.data.copy_(source.get_tensor("linear.weight"))
    return TextTower(transformer, dense).eval()


def load_vision_tower(weights: Path) -> tuple[VisionTower, CLIPModel]:
    model = CLIPModel.from_pretrained(weights, local_files_only=True).eval()
    return VisionTower(model).eval(), model
