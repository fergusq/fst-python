# PyPykko

PyPykko is a wrapper around [pykko](https://github.com/pkauppin/pykko). It provides the basic analysis and generation API in an easily installable package.
PyPykko can be installed without compiling anything (as the transducers are pre-compiled) or pulling in any native dependencies (such as hfst).

This package contains (slightly modified for kfst compatibility versions of) all the files in the tools directory of pykko as well as constants.py and file\_tools.py from the scripts directory and utils.py from the scripts directory as scriptutils.py. It also provides the novel reinflect.py and extras.py.

## Installation

PyPykko is available on PyPI and can be installed with pip:

```sh
pip install pypykko
```

## Usage

There are two main Python methods `utils.analyze` and `generate.generate_wordform` inherited from Pykko proper; besides these there is `reinflect.reinflect` that is perhaps a more suitable interface for general reinflection. There is also bolted-on alignment support in `extras.analyze_with_compound_parts`.

### reinflect.reinflect

`reinflect.reinflect` tries to reinflect a word to the best of its ability. It can be instructed either with a model word or with a specific form. Further, it can be given the form the original word was in if known ahead of time and the part-of-speech of the word.

```py
>>> from pypykko.reinflect import reinflect
>>> reinflect("mökkiammeemme", model="talossa")
{'mökkiammeessa'}
>>> reinflect("esijuosta", model="katselemme")
{'esijuoksemme'}
>>> reinflect("mökkiammeemme", new_form="+sg+nom")
{'mökkiamme'}
>>> reinflect("möhkö", new_form="+pl+ine+ko")
{'möhköissäkö'}
>>> reinflect("viinissä", model="talot")
{'viinet'}
>>> reinflect("viinissä", model="talot", orig_form="+sg+ine")
{'viinit'}
>>> reinflect("hömppäämme", model="juokset", pos="verb")
{'hömppäät'}
>>> reinflect("hömppäämme", model="juokset", pos="noun")
{'hömpät'}
```


### utils.analyze and extras.analyze\_with\_compound\_parts

`utils.analyze` should be used in most cases:

```py
>>> from pypykko.utils import analyze
>>> analyze("hätkähtäneet")
[('hätkähtäneet', 'Lexicon', 'hätkähtää', 'verb', '', '', '+past+conneg+pl', 0.0), ('hätkähtäneet', 'Lexicon', 'hätkähtää', 'verb', '', '', '+part_past+pl+nom', 0.0)]
```

The fields of the outcoming tuple are:

1. Surface form (input as it is given)
2. The source of the word: eg. `Lexicon` if it is a word known ahead of time, `Guesser|Any` for unknown words and `Lexicon|Pfx` for words analyzed as the compounds of known words.
3. The lemma form of the word; notably this can contain pipe symbols to delimit compound parts: `ilma|luukku`. Sometimes Finnish has infix inflection, and the compound parts can be separately inflected (eg. `uudenvuoden` -> `uusi|vuosi`).
4. The part of speech of the word.
5. The homonym number of the word (can be empty). Eg. the word viini has two senses that have slightly different inflection: wine (viini -> viinin) and quiver (viini -> viinen). In cases where such homonyms exist but it is impossible to tell which form is presented (the nominative form viini here), we get both interpretations:
```
[('viini', 'Lexicon', 'viini', 'noun', '1', '', '+sg+nom', 0.0), ('viini', 'Lexicon', 'viini', 'noun', '2', '', '+sg+nom', 0.0)]
```
In cases where the form is unambiguous (eg. viinen), we get only the homonym number that is relevant:
```
[('viinen', 'Lexicon', 'viini', 'noun', '2', '', '+sg+gen', 0.0)]
```
In cases where the homonym is different in different interpretations, we get annotated interpretations:
```
[('viinin', 'Lexicon', 'viini', 'noun', '2', '', '+pl+ins', 0.0), ('viinin', 'Lexicon', 'viini', 'noun', '1', '', '+sg+gen', 0.0)]
```
6. Register annotation, eg:
```
>>> analyze("höpsöillä")
[('höpsöillä', 'Lexicon', 'höpsö', 'noun', '', '⟨coll⟩', '+pl+ade', 0.0), ('höpsöillä', 'Lexicon', 'höpsö', 'adjective', '', '⟨coll⟩', '+pl+ade', 0.0)]
```
7. Morphological tags that name the inflectional form.

`extras.analyze\_with\_compound\_parts` is of use when it is useful to know the exact inflected forms of the compound parts of a word.
Eg. when looking at "isonvarpaan", one might want to not only know that it is the compound of "iso" and "varvas" but also that they are in the forms "ison" and "varpaan".
`extras.anlyze\_with\_compound\_parts` returns the character ranges matching compound parts.

```
>>> analyze_with_compound_parts("isonvarpaan")
('isonvarpaan', 'Lexicon', 'iso|varvas', 'noun', '', '', '+sg+gen', 0.0, (range(0, 4), range(4, 11)))
```

### generate.generate\_wordform

`generate\_wordform` is a simple-to-use api to inflect in-lexicon words.

```py
>>> from pypykko.generate import generate_wordform
>>> generate_wordform("höpönassu", "noun", '+pl+abe+ko')
{'höpönassuittako'}
```


## License

PyPykko is licensed under the MIT license like Pykko itself, as it is mostly constituted of Pykko's files with minor modifications. See the LICENSE file for details. Note that kfst (and kfst-rs) have less permissive licenses.

Files from Pykko itself are modified from the version in commit 95f3d51f0e94a1e88ab7c750f2bedcb6b3fd5edd. The compiled transducers are from the same commit.
