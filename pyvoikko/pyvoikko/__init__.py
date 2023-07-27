from __future__ import annotations

import os.path

from kfst import FST, TokenizationException

from .analysis import VoikkoAnalysis

SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))

VOIKKO_FST = FST.from_kfst_file(os.path.join(SCRIPT_DIR, "voikko.kfst"))

def analyse(word: str) -> list[VoikkoAnalysis]:
    """
    Analyses a word using voikko.

    Args:
        word: word to analyse
    
    Returns:
        list of `VoikkoAnalysis` objects
    """
    ans = []
    try:
        for analysis, _weight in VOIKKO_FST.lookup(word):
            ans.append(VoikkoAnalysis.from_voikko_analysis(analysis))
    except TokenizationException:
        pass
    
    return ans