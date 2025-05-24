#!./venv/bin/python
import hfst
import kfst_py
import kfst
assert kfst.BACKEND == "kfst_rs"
import timeit

paragraph = [
        "on",
        "maanantaiaamu",
        "heinäkuussa",
        "aurinko",
        "paiskaa",
        "niin",
        "lämpöisesti",
        "heikon",
        "tuulen",
        "avulla",
        "ja",
        "peipposet",
        "kajahuttelevat",
        "ensimmäisiä",
        "kovia",
        "säveleitään",
        "tuoksuavissa",
        "koivuissa",
        "kirkon",
        "itäisellä",
        "seinuksella",
        "on",
        "kivipenkki",
        "juuri",
        "nyt",
        "saapuu",
        "keski-ikäinen",
        "työmies",
        "ja",
        "istuutuu",
        "penkille",
        "hän",
        "näyttää",
        "väsyneeltä",
        "alakuloiselta",
        "haluttomalla",
        "aivan",
        "kuin",
        "olisi",
        "vastikään",
        "tullut",
        "perheellisestä",
        "riidasta",
        "tahi",
        "jättänyt",
        "eilisen",
        "sapatinpäivän",
        "pyhittämättä",
    ]
gold = [
        [("[Lt][Xp]olla[X]o[Tt][Ap][P3][Ny][Ef]n", 0)],
        [("[Ln][Xp]maanantai[X]maanantai[Sn][Ny][Bh][Bc][Ln][Xp]aamu[X]aamu[Sn][Ny]", 0)],
        [("[Ln][Xp]heinä[X]hein[Sn][Ny]ä[Bh][Bc][Ln][Xp]kuu[X]kuu[Sine][Ny]ssa", 0)],
        [("[Ln][Xp]aurinko[X]aurinko[Sn][Ny]", 0), ("[Lem][Xp]Aurinko[X]aurinko[Sn][Ny]", 0), ("[Lee][Xp]Auri[X]aur[Sg][Ny]in[Fko][Ef]ko", 0)],
        [("[Lt][Xp]paiskata[X]paiska[Tt][Ap][P3][Ny][Eb]a", 0)],
        [("[Ls][Xp]niin[X]niin", 0)],
        [("[Ln][Xp]lämpö[X]lämpö[Ll][Xj]inen[X]ise[Ssti]sti", 0)],
        [("[Ll][Xp]heikko[X]heiko[Sg][Ny]n", 0)],
        [("[Ln][Xp]tuuli[X]tuul[Sg][Ny]en", 0)],
        [("[Ln][Xp]avu[X]avu[Sade][Ny]lla", 0), ("[Ln][Xp]apu[X]avu[Sade][Ny]lla", 0)],
        [("[Lc][Xp]ja[X]ja", 0)],
        [("[Ln][Xp]peipponen[X]peippo[Sn][Nm]set", 0)],
        [],
        [("[Lu][Xp]ensimmäinen[X]ensimmäi[Sp][Nm]siä", 0)],
        [("[Lnl][Xp]kova[X]kov[Sp][Nm]ia", 0)],
        [],
        [],
        [("[Ln][Xp]koivu[X]koivu[Sine][Nm]issa", 0), ("[Les][Xp]Koivu[X]koivu[Sine][Nm]issa", 0)],
        [("[Ln][Ica][Xp]kirkko[X]kirko[Sg][Ny]n", 0)],
        [("[Ln][De][Xp]itä[X]itä[Ll][Xj]inen[X]ise[Sade][Ny]llä", 0)],
        [("[Ln][Xp]seinus[X]seinukse[Sade][Ny]lla", 0)],
        [("[Lt][Xp]olla[X]o[Tt][Ap][P3][Ny][Ef]n", 0)],
        [("[Ln][Ica][Xp]kivi[X]kiv[Sn][Ny]i[Bh][Bc][Ln][Xp]penkki[X]penkk[Sn][Ny]i", 0)],
        [("[Ln][Xp]juuri[X]juur[Sn][Ny]i", 0), ("[Ls][Xp]juuri[X]juuri", 0), ("[Lt][Xp]juuria[X]juuri[Tk][Ap][P2][Ny][Eb]", 0), ("[Lt][Xp]juuria[X]juur[Tt][Ai][P3][Ny][Ef]i", 0)],
        [("[Ls][Xp]nyt[X]nyt", 0)],
        [("[Lt][Xp]saapua[X]saapuu[Tt][Ap][P3][Ny][Ef]", 0)],
        [("[Lp]keski[De]-[Bh][Bc][Ln][Xp]ikä[X]ikä[Ll][Xj]inen[X]i[Sn][Ny]nen", 0)],
        [("[Ln][Xp]työ[X]työ[Sn][Ny][Bh][Bc][Ln][Xp]mies[X]mies[Sn][Ny]", 0)],
        [("[Lc][Xp]ja[X]ja", 0)],
        [("[Lt][Xp]istuutua[X]istuutuu[Tt][Ap][P3][Ny][Ef]", 0)],
        [("[Ln][Xp]penkki[X]penki[Sall][Ny]lle", 0)],
        [("[Lr][Xp]hän[X]hä[Sn][Ny]n", 0)],
        [("[Lt][Xp]näyttää[X]näyttä[Tn1][Eb]ä", 0), ("[Lt][Xp]näyttää[X]näytt[Tt][Ap][P3][Ny][Ef]ää", 0)],
        [("[Lt][Irm][Xp]väsyä[X]väsy[Ll][Ru]n[Xj]yt[X]ee[Sabl][Ny]ltä", 0)],
        [("[Ln][De][Xp]ala[X]al[Sn][Ny]a[Bh][Bc][Lnl][Xp]kulo[X]kulo[Ll][Xj]inen[X]ise[Sabl][Ny]lta", 0)],
        [("[Ln][Xp]halu[X]halu[Ll][Xj]ton[X]ttoma[Sade][Ny]lla", 0)],
        [("[Ls][Xp]aivan[X]aivan", 0)],
        [("[Lc][Xp]kuin[X]kuin", 0), ("[Ln][Xp]kuu[X]ku[Sin][Nm]in", 0)],
        [("[Lt][Xp]olla[X]ol[Te][Ap][P3][Ny][Eb]isi", 0)],
        [("[Ls][Xp]vast=ikään[X]vast[Bm]ikään", 0)],
        [("[Lt][Xp]tulla[X]tul[Ll][Ru]l[Xj]ut[X][Sn][Ny]ut", 0), ("[Lt][Xp]tulla[X]tul[Ll][Rt][Xj]tu[X]lu[Sn][Nm]t", 0)],
        [("[Ln][Xp]perhe[X]perhee[Ll]lli[Xj]nen[X]se[Sela][Ny]stä", 0)],
        [("[Ln][Xp]riita[X]riida[Sela][Ny]sta", 0)],
        [("[Lc][Xp]tahi[X]tahi", 0)],
        [("[Lt][Xp]jättää[X]jättä[Ll][Ru]n[Xj]yt[X][Sn][Ny]yt", 0)],
        [("[Lnl][Xp]eilinen[X]eili[Sg][Ny]sen", 0)],
        [("[Ln][Xp]sapatti[X]sapat[Sg][Ny]in[Bh][Bc][Ln][Xp]päivä[X]päiv[Sg][Ny]än", 0)],
        [("[Lt][Xp]pyhittää[X]pyhittä[Ln]m[Xj]ä[X][Rm]ä[Sab][Ny]ttä", 0), ("[Lt][Xp]pyhittää[X]pyhittä[Tn3][Ny][Sab]mättä", 0)],
    ]

def test(fst):
    for text in paragraph:
        list(fst.lookup(text))

pyfst = kfst_py.transducer.FST.from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst")
rsfst = kfst.transducer.FST.from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst")
istr = hfst.HfstInputStream("voikko.hfst.ol")
hyfst = istr.read()
istr.close()

print("pyfst:")

print(timeit.timeit(lambda: test(pyfst), number=100))

print("rsfst:")

print(timeit.timeit(lambda: test(rsfst), number=100))

print("hyfst:")

print(timeit.timeit(lambda: test(hyfst), number=100))
