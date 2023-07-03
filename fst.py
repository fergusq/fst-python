from pathlib import Path
import argparse
from collections import defaultdict

class FST:
    final_states: dict[int, float] = {}
    rules: defaultdict[int, defaultdict[str, list[tuple[int, str, int]]]] = defaultdict(lambda: defaultdict(list))
    symbols: set[str] = set()

    def split_to_symbols(self, text: str):
        slist = sorted(self.symbols, key=lambda x: -len(x))
        while text:
            for s in slist:
                if text.startswith(s):
                    yield s
                    text = text[len(s):]
                    break

            else:
                return

    def run_fst(self, input_symbols: list[str], output_symbols: list[str] = [], state=0):
        transitions = self.rules[state]
        if not input_symbols:
            if state in self.final_states:
                yield output_symbols
                return

        else:
            s = input_symbols[0]
            for next_state, osymbol, weight in transitions[s]:
                #print(state, "->", next_state, s, osymbol, input_symbols, output_symbols)
                o = [osymbol] if osymbol != "@0@" else []
                yield from self.run_fst(input_symbols[1:], output_symbols=output_symbols + o, state=next_state)

        for next_state, osymbol, weight in transitions["@0@"]:
            #print(state, "->", next_state, "@0@", osymbol, input_symbols, output_symbols)
            o = [osymbol] if osymbol != "@0@" else []
            yield from self.run_fst(input_symbols, output_symbols=output_symbols + o, state=next_state)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("fst_file", type=Path)
    args = parser.parse_args()

    fst = FST()
    for line in args.fst_file.read_text().splitlines():
        fields = line.split("\t")
        if len(fields) == 1:
            fst.final_states[int(fields[0])] = 0
        
        elif len(fields) == 2:
            fst.final_states[int(fields[0])] = float(fields[1])

        elif len(fields) == 4:
            fst.rules[int(fields[0])][fields[2]].append((int(fields[1]), fields[3], 0))
            fst.symbols.add(fields[2])
            fst.symbols.add(fields[3])

        elif len(fields) == 5:
            fst.rules[int(fields[0])][fields[2]].append((int(fields[1]), fields[3], float(fields[4])))
            fst.symbols.add(fields[2])
            fst.symbols.add(fields[3])

    while True:
        try:
            text = input("> ")

        except EOFError:
            break

        for os in fst.run_fst(list(fst.split_to_symbols(text))):
            print("".join(os))

if __name__ == "__main__":
    main()

