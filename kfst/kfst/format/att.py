from collections import defaultdict

from ..transducer import FST

def decode_att(att_code: str) -> FST:
    """
    Parses a transducer in A&AT format and returns an FST object.
    """

    final_states: dict[int, float] = {}
    rules: defaultdict[int, defaultdict[str, list[tuple[int, str, float]]]] = defaultdict(lambda: defaultdict(list))
    symbols: set[str] = set()
    for line in att_code.splitlines():
        fields = line.split("\t")
        if len(fields) == 1:
            final_states[int(fields[0])] = 0
        
        elif len(fields) == 2:
            final_states[int(fields[0])] = float(fields[1])

        elif len(fields) == 4:
            rules[int(fields[0])][fields[2]].append((int(fields[1]), fields[3], 0))
            symbols.add(fields[2])
            symbols.add(fields[3])

        elif len(fields) == 5:
            rules[int(fields[0])][fields[2]].append((int(fields[1]), fields[3], float(fields[4])))
            symbols.add(fields[2])
            symbols.add(fields[3])

        else:
            raise RuntimeError("Cannot parse line:", line)

    return FST.from_rules(final_states, rules, symbols)