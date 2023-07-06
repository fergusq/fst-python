import os.path

from .omorfi import Omorfi

SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))

def load_omorfi(
        load_analyser=True,
        load_segmenter=True,
        load_generator=True,
        large_coverage=True,
):
    """
    Loads the built-in omorfi FSA language models.

    Args:
        load_analyser:  set to false to skip loading the analyser
        load_segmenter: set to false to skip loading the segmenter
        load_generator: set to false to skip loading the generator
        large_coverage: set to false to load the smaller coverage language model

    Returns:
        `Omorfi` object

    See also:
        http://flammie.github.io/omorfi/smaller-lexicons.html
    """
    omorfi_obj = Omorfi()
    analyser_file = "omorfi.describe.kfst" if large_coverage else "omorfi.analyse.kfst"
    if load_analyser:
        omorfi_obj.load_analyser(os.path.join(SCRIPT_DIR, analyser_file))
    
    if load_segmenter:
        omorfi_obj.load_segmenter(os.path.join(SCRIPT_DIR, "omorfi.segment.kfst"))
    
    if load_generator:
        omorfi_obj.load_generator(os.path.join(SCRIPT_DIR, "omorfi.generate.kfst"))
    
    return omorfi_obj