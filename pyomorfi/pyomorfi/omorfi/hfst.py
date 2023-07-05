#!/usr/bin/env python
# -*- coding: utf-8 -*-


"""
HFST utils.
"""

from kfst import FST


def load_hfst(f):
    """Load an AT&T language model from file, with some error handling.

    Args:
        f:  containing single hfst automaton binary.

    Throws:
        FileNotFoundError if file is not found
    """
    return FST.from_att_file(f)
