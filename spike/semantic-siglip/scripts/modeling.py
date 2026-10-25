from __future__ import annotations

from pathlib import Path

import torch
from torch import nn
from torch.nn import functional as F
from transformers import SiglipModel

EMBEDDING_DIM = 768
CONTEXT_LENGTH = 64
IMAGE_SIZE = 512


class TextTower(nn.Module):
    def __init__(self, model: SiglipModel) -> None:
        super().__init__()
        self.text_model = model.text_model

    def forward(self, input_ids: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        raw = self.text_model(input_ids=input_ids).pooler_output
        return raw, F.normalize(raw, p=2, dim=-1)


class VisionTower(nn.Module):
    def __init__(self, model: SiglipModel) -> None:
        super().__init__()
        self.vision_model = model.vision_model

    def forward(self, pixel_values: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        raw = self.vision_model(pixel_values=pixel_values).pooler_output
        return raw, F.normalize(raw, p=2, dim=-1)


def load_towers(weights: Path) -> tuple[TextTower, VisionTower, SiglipModel]:
    model = SiglipModel.from_pretrained(weights, local_files_only=True).eval()
    text = model.config.text_config
    vision = model.config.vision_config
    if text.max_position_embeddings != CONTEXT_LENGTH:
        raise RuntimeError(f"Unexpected text length: {text.max_position_embeddings}")
    if text.hidden_size != EMBEDDING_DIM or text.projection_size != EMBEDDING_DIM:
        raise RuntimeError("Unexpected text hidden/projection dimensions")
    if model.text_model.head.out_features != EMBEDDING_DIM:
        raise RuntimeError("Text projection head is missing or has the wrong dimension")
    if vision.image_size != IMAGE_SIZE or vision.patch_size != 16:
        raise RuntimeError("Unexpected vision resolution or patch size")
    return TextTower(model).eval(), VisionTower(model).eval(), model
