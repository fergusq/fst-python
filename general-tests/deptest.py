import pyomorfi
import pyvoikko

o = pyomorfi.load_omorfi()
assert [x.raw for x in o.analyse("kissakalojemme")] == ['[WORD_ID=kissa][UPOS=NOUN][NUM=SG][CASE=NOM][BOUNDARY=COMPOUND][WORD_ID=kala][UPOS=NOUN][NUM=PL][CASE=GEN][POSS=PL1][WEIGHT=0.000000]', '[WORD_ID=kissakala][UPOS=NOUN][NUM=PL][CASE=GEN][POSS=PL1][WEIGHT=0.000000]']

assert [x.FSTOUTPUT for x in pyvoikko.analyse("kissakalojemme")] == ['[Ln][Xp]kissa[X]kiss[Sn][Ny]a[Bh][Bc][Ln][Xp]kala[X]kal[Sg][Nm]oje[O1m]mme']
