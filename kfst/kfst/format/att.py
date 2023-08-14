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

def encode_att(fst: FST) -> str:
    """
    Encodes the FST in the AT&T tabular format.
    """

    ans = []
    for final_state, weight in fst.final_states.items():
        if weight == 0:
            ans.append(str(final_state))
        else:
            ans.append("\t".join([str(final_state), str(weight)]))
    
    for from_state, transitions in fst.rules.items():
        for input_symbol, transitions_list in transitions.items():
            for to_state, output_symbol, weight in transitions_list:
                if weight == 0:
                    ans.append("\t".join([str(from_state), str(to_state), input_symbol, output_symbol]))
                else:
                    ans.append("\t".join([str(from_state), str(to_state), input_symbol, output_symbol, str(weight)]))
    
    return "\n".join(ans)