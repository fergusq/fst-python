from pypykko.extras import analyze_with_compound_parts

for word, ranges in [("isonvarpaan", (range(0, 4), range(4, 11))),
                     ("löylykauha", (range(0, 5), range(5, 10))),
                     ("palautevyöryn", (range(0, 7), range(7, 13))),
                     ("Mielenterveyspulmiin", (range(0, 6), range(6, 13), range(13, 20))),
                     ("juoksemmeko", (range(0, 11),)),
                     ("esijuoksemmeko", (range(0, 3), range(3, 14))),
                     ("kissa", (range(0, 5),))]:

    sys_ranges = analyze_with_compound_parts(word)[0][-1]
    assert sys_ranges == ranges, f"Expected {ranges} got {sys_ranges}"
