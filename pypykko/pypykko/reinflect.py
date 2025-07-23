from pypykko.generate import generator_fst
from pypykko.utils import PARSER_FST
from collections import defaultdict

def reinflect(original: str, new_form: str | None = None, model: str | None = None, pos: str | None = None, orig_form: str | None = None) -> set[str]:

    assert (new_form is None) != (model is None), "Must provide either new_form or model"

    # Get ordered deduplicated sequence of target form strings using dict trick

    if new_form is not None:
        target_forms = {new_form: None}
    else:
        target_forms = {analysis[0].split("\t")[-1]: None for analysis in PARSER_FST.lookup(model) if (pos is None or pos == analysis[0].split("\t")[2])}

    # Find analyses that match the filter

    analyses: list[tuple[str, float]] = [analysis for analysis in PARSER_FST.lookup(original) if (orig_form is None or orig_form == analysis[0].split("\t")[-1]) and (pos is None or pos == analysis[0].split("\t")[2])]
        
    # Try to inflect from best to worst

    results = defaultdict(lambda: set())

    for target_form in target_forms:
        for analysis in analyses:
            body = analysis[0].split("\t")
            input_string = f"{body[0]}^TAB{body[1]}^TAB^{body[2]}^TAB{body[3]}^TAB^TAB{target_form}"
            for output, weight in generator_fst.lookup(input_string):
                results[weight].add(output)
            if len(results) > 0:
                return set(results[min(results.keys())])
    
    # Nothing stuck

    return set()
    