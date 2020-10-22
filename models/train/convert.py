#! /usr/bin/env python3
"""Convert between python and rust model format."""

from pathlib import Path
import os
import numpy as np
import torch
import subprocess
from tqdm import tqdm
import coloredlogs
import logging

# Setup logging
logger = logging.getLogger(__name__)
coloredlogs.install(level='INFO', logger=logger,
                    fmt="%(levelname)s %(message)s")

ROOT_PATH = Path('../fairy-safe')

config_path = ROOT_PATH / 'config.json'
vocab_path = ROOT_PATH / 'vocab.json'
merges_path = ROOT_PATH / 'merges.txt'
weights_path = ROOT_PATH / 'pytorch_model.bin'

logger.info("Loading weights")
weights = torch.load(weights_path, map_location='cpu')
logger.info("Processing weights")
nps = {}
for k, v in tqdm(weights.items()):
    nps[k] = np.ascontiguousarray(v.cpu().numpy()).astype(np.float32)
    if k == 'wte.weight':
        nps['lm_head.weight'] = np.ascontiguousarray(v.cpu().numpy()).astype(
            np.float32)

logger.info("Saving np weights")
np.savez(ROOT_PATH / 'model.npz', **nps)

source = str(ROOT_PATH / 'model.npz')
target = str(ROOT_PATH / 'model.ot')

logger.info("Converting weights")
subprocess.call(['convert-tensor', source, target])

os.remove(str(ROOT_PATH / 'model.npz'))
