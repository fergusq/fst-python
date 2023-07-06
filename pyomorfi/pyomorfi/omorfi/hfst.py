#!/usr/bin/env python
# -*- coding: utf-8 -*-


"""
HFST utils.
"""

from pathlib import Path

from kfst import FST


def load_hfst(f):
    """Load an KFST language model from file, with some error handling.

    Args:
        f:  containing single hfst automaton binary.

    Throws:
        FileNotFoundError if file is not found
    """
    p = Path(f)
    if p.suffix == ".att":
        return FST.from_att_file(f)
    return FST.from_kfst_file(f)
