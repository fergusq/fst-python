use criterion::{criterion_group, criterion_main, Criterion};
use kfst_rs::{InternalFSTState, FST};
use std::hint::black_box;

fn load_pypykko(c: &mut Criterion) {
    c.bench_function("load pypykko", |b| {
        b.iter(|| {
            black_box(FST::from_kfst_file(
                black_box("../pypykko/pypykko/fi-parser.kfst".to_string()),
                false,
            ))
        })
    });
}

fn run_voikko(c: &mut Criterion) {
    let paragraph = [
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
    ];
    let gold: [Vec<(&'static str, f64)>; 48] = [
        vec![("[Lt][Xp]olla[X]o[Tt][Ap][P3][Ny][Ef]n", 0.0)],
        vec![("[Ln][Xp]maanantai[X]maanantai[Sn][Ny][Bh][Bc][Ln][Xp]aamu[X]aamu[Sn][Ny]", 0.0)],
        vec![("[Ln][Xp]heinä[X]hein[Sn][Ny]ä[Bh][Bc][Ln][Xp]kuu[X]kuu[Sine][Ny]ssa", 0.0)],
        vec![("[Ln][Xp]aurinko[X]aurinko[Sn][Ny]", 0.0), ("[Lem][Xp]Aurinko[X]aurinko[Sn][Ny]", 0.0), ("[Lee][Xp]Auri[X]aur[Sg][Ny]in[Fko][Ef]ko", 0.0)],
        vec![("[Lt][Xp]paiskata[X]paiska[Tt][Ap][P3][Ny][Eb]a", 0.0)],
        vec![("[Ls][Xp]niin[X]niin", 0.0)],
        vec![("[Ln][Xp]lämpö[X]lämpö[Ll][Xj]inen[X]ise[Ssti]sti", 0.0)],
        vec![("[Ll][Xp]heikko[X]heiko[Sg][Ny]n", 0.0)],
        vec![("[Ln][Xp]tuuli[X]tuul[Sg][Ny]en", 0.0)],
        vec![("[Ln][Xp]avu[X]avu[Sade][Ny]lla", 0.0), ("[Ln][Xp]apu[X]avu[Sade][Ny]lla", 0.0)],
        vec![("[Lc][Xp]ja[X]ja", 0.0)],
        vec![("[Ln][Xp]peipponen[X]peippo[Sn][Nm]set", 0.0)],
        vec![],
        vec![("[Lu][Xp]ensimmäinen[X]ensimmäi[Sp][Nm]siä", 0.0)],
        vec![("[Lnl][Xp]kova[X]kov[Sp][Nm]ia", 0.0)],
        vec![],
        vec![],
        vec![("[Ln][Xp]koivu[X]koivu[Sine][Nm]issa", 0.0), ("[Les][Xp]Koivu[X]koivu[Sine][Nm]issa", 0.0)],
        vec![("[Ln][Ica][Xp]kirkko[X]kirko[Sg][Ny]n", 0.0)],
        vec![("[Ln][De][Xp]itä[X]itä[Ll][Xj]inen[X]ise[Sade][Ny]llä", 0.0)],
        vec![("[Ln][Xp]seinus[X]seinukse[Sade][Ny]lla", 0.0)],
        vec![("[Lt][Xp]olla[X]o[Tt][Ap][P3][Ny][Ef]n", 0.0)],
        vec![("[Ln][Ica][Xp]kivi[X]kiv[Sn][Ny]i[Bh][Bc][Ln][Xp]penkki[X]penkk[Sn][Ny]i", 0.0)],
        vec![("[Ln][Xp]juuri[X]juur[Sn][Ny]i", 0.0), ("[Ls][Xp]juuri[X]juuri", 0.0), ("[Lt][Xp]juuria[X]juuri[Tk][Ap][P2][Ny][Eb]", 0.0), ("[Lt][Xp]juuria[X]juur[Tt][Ai][P3][Ny][Ef]i", 0.0)],
        vec![("[Ls][Xp]nyt[X]nyt", 0.0)],
        vec![("[Lt][Xp]saapua[X]saapuu[Tt][Ap][P3][Ny][Ef]", 0.0)],
        vec![("[Lp]keski[De]-[Bh][Bc][Ln][Xp]ikä[X]ikä[Ll][Xj]inen[X]i[Sn][Ny]nen", 0.0)],
        vec![("[Ln][Xp]työ[X]työ[Sn][Ny][Bh][Bc][Ln][Xp]mies[X]mies[Sn][Ny]", 0.0)],
        vec![("[Lc][Xp]ja[X]ja", 0.0)],
        vec![("[Lt][Xp]istuutua[X]istuutuu[Tt][Ap][P3][Ny][Ef]", 0.0)],
        vec![("[Ln][Xp]penkki[X]penki[Sall][Ny]lle", 0.0)],
        vec![("[Lr][Xp]hän[X]hä[Sn][Ny]n", 0.0)],
        vec![("[Lt][Xp]näyttää[X]näyttä[Tn1][Eb]ä", 0.0), ("[Lt][Xp]näyttää[X]näytt[Tt][Ap][P3][Ny][Ef]ää", 0.0)],
        vec![("[Lt][Irm][Xp]väsyä[X]väsy[Ll][Ru]n[Xj]yt[X]ee[Sabl][Ny]ltä", 0.0)],
        vec![("[Ln][De][Xp]ala[X]al[Sn][Ny]a[Bh][Bc][Lnl][Xp]kulo[X]kulo[Ll][Xj]inen[X]ise[Sabl][Ny]lta", 0.0)],
        vec![("[Ln][Xp]halu[X]halu[Ll][Xj]ton[X]ttoma[Sade][Ny]lla", 0.0)],
        vec![("[Ls][Xp]aivan[X]aivan", 0.0)],
        vec![("[Lc][Xp]kuin[X]kuin", 0.0), ("[Ln][Xp]kuu[X]ku[Sin][Nm]in", 0.0)],
        vec![("[Lt][Xp]olla[X]ol[Te][Ap][P3][Ny][Eb]isi", 0.0)],
        vec![("[Ls][Xp]vast=ikään[X]vast[Bm]ikään", 0.0)],
        vec![("[Lt][Xp]tulla[X]tul[Ll][Ru]l[Xj]ut[X][Sn][Ny]ut", 0.0), ("[Lt][Xp]tulla[X]tul[Ll][Rt][Xj]tu[X]lu[Sn][Nm]t", 0.0)],
        vec![("[Ln][Xp]perhe[X]perhee[Ll]lli[Xj]nen[X]se[Sela][Ny]stä", 0.0)],
        vec![("[Ln][Xp]riita[X]riida[Sela][Ny]sta", 0.0)],
        vec![("[Lc][Xp]tahi[X]tahi", 0.0)],
        vec![("[Lt][Xp]jättää[X]jättä[Ll][Ru]n[Xj]yt[X][Sn][Ny]yt", 0.0)],
        vec![("[Lnl][Xp]eilinen[X]eili[Sg][Ny]sen", 0.0)],
        vec![("[Ln][Xp]sapatti[X]sapat[Sg][Ny]in[Bh][Bc][Ln][Xp]päivä[X]päiv[Sg][Ny]än", 0.0)],
        vec![("[Lt][Xp]pyhittää[X]pyhittä[Ln]m[Xj]ä[X][Rm]ä[Sab][Ny]ttä", 0.0), ("[Lt][Xp]pyhittää[X]pyhittä[Tn3][Ny][Sab]mättä", 0.0)],
    ];
    let voikko =
        FST::from_kfst_file("../pyvoikko/pyvoikko/voikko.kfst".to_string(), false).unwrap();
    c.bench_function("run voikko", |b| {
        b.iter(|| {
            for (input, gold_val) in paragraph.iter().zip(gold.iter()) {
                let mut analysis = black_box(voikko.lookup(
                    black_box(input),
                    black_box(InternalFSTState::default()),
                    black_box(true),
                ))
                .unwrap();
                analysis.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap().then(x.0.cmp(&y.0)));
                let mut mut_gold = gold_val.clone();
                mut_gold.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap().then(x.0.cmp(&y.0)));

                for (a, b) in analysis.into_iter().zip(mut_gold) {
                    assert!(a.0.as_str() == b.0);
                    assert!(a.1 == b.1);
                }
            }
        })
    });
}

criterion_group!(benches, load_pypykko, run_voikko);

criterion_main!(benches);
