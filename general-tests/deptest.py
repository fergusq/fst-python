import pyomorfi
import pyvoikko

o = pyomorfi.load_omorfi()
assert [x.raw for x in o.analyse("kissakalojemme")] == ['[WORD_ID=kissa][UPOS=NOUN][NUM=SG][CASE=NOM][BOUNDARY=COMPOUND][WORD_ID=kala][UPOS=NOUN][NUM=PL][CASE=GEN][POSS=PL1][WEIGHT=0.000000]', '[WORD_ID=kissakala][UPOS=NOUN][NUM=PL][CASE=GEN][POSS=PL1][WEIGHT=0.000000]']

assert [x.FSTOUTPUT for x in pyvoikko.analyse("kissakalojemme")] == ['[Ln][Xp]kissa[X]kiss[Sn][Ny]a[Bh][Bc][Ln][Xp]kala[X]kal[Sg][Nm]oje[O1m]mme']

from pypykko.reinflect import reinflect
assert reinflect("mökkiammeemme", model="talossa") == {"mökkiammeessa"}
assert reinflect("esijuosta", model="katselemme") == {'esijuoksemme'}
assert reinflect("mökkiammeemme", new_form="+sg+nom") == {'mökkiamme'}
assert reinflect("möhkö", new_form="+pl+ine+ko") == {'möhköissäkö'}
assert reinflect("viinissä", model="talot") == {'viinet'}
assert reinflect("viinissä", model="talot", orig_form="+sg+ine") == {'viinit'}
assert reinflect("hömppäämme", model="juokset", pos="verb") == {'hömppäät'}
assert reinflect("hömppäämme", model="juokset", pos="noun") == {'hömpät'}

from pypykko.utils import analyze
assert analyze("hätkähtäneet") == [('hätkähtäneet', 'Lexicon', 'hätkähtää', 'verb', '', '', '+past+conneg+pl', 0.0), ('hätkähtäneet', 'Lexicon', 'hätkähtää', 'verb', '', '', '+part_past+pl+nom', 0.0)]

from pypykko.generate import generate_wordform
assert generate_wordform("höpönassu", "noun", '+pl+abe+ko') == {'höpönassuittako'}
