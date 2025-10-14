import os
from .file_tools import read_list
import kfst
from .constants import GENERATOR_FST_PATH, TAB
from .scriptutils import is_valid_pos, is_uninflectable
from .utils import add_compound_separators, inf, pos_tag

CURR = os.path.dirname(__file__)

POS_FST_SOURCES = {
	'noun-pl':          ['Lexicon', 'Lexicon|Pfx', 'Lexicon|Hyp'],
	'noun':             ['Lexicon', 'Lexicon|Pfx', 'Lexicon|Hyp'],
	'proper-pl':        ['Lexicon', 'Lexicon|Pfx', 'Lexicon|Hyp'],
	'proper':           ['Lexicon', 'Lexicon|Pfx', 'Lexicon|Hyp'],
	'adjective':        ['Lexicon', 'Lexicon|Pfx', 'Lexicon|Hyp'],
	'pronoun':          ['Lexicon', 'Lexicon|Hyp'],
	'pronoun-pl':       ['Lexicon', 'Lexicon|Hyp'],
	'verb':             ['Lexicon', 'Lexicon|Pfx'],
	'participle':       ['Lexicon', 'Lexicon|Pfx'],
	'numeral':          ['Lexicon|Num', 'Lexicon', 'Guesser|Any'],  # (!)
	'ordinal':          ['Lexicon|Num', 'Lexicon', 'Guesser|Any'],  # (!)
	'adverb':           ['Lexicon'],
	'adposition':       ['Lexicon'],
	'interjection':     ['Lexicon'],
	'conjunction':      ['Lexicon'],
	'conjunction+verb': ['Lexicon'],
	'adverb+verb':      ['Lexicon'],
	'none':             ['Lexicon'],
}

HOMONYMOUS = {
	("ahtaus", "noun"),
	("ale", "noun"),
	("appi", "noun"),
	("g", "noun"),
	("haiku", "noun"),
	("halata", "verb"),
	("hepo", "noun"),
	("isota", "verb"),
	("joka", "pronoun"),
	("karvaus", "noun"),
	("keritä", "verb"),
	("koto", "noun"),
	("kuori", "noun"),
	("kuti", "noun"),
	("l", "noun"),
	("lahti", "noun"),
	("laki", "noun"),
	("lento", "noun"),
	("live", "noun"),
	("m", "noun"),
	("merirosvous", "noun"),
	("mutu", "noun"),
	("palvi", "noun"),
	("parka", "noun"),
	("peitsi", "noun"),
	("pokata", "verb"),
	("puola", "noun"),
	("raakata", "verb"),
	("raita", "noun"),
	("raueta", "verb"),
	("ripsi", "noun"),
	("riuku", "noun"),
	("rosvous", "noun"),
	("s", "noun"),
	("saksi", "noun"),
	("sietä", "verb"),
	("siivous", "noun"),
	("sini", "noun"),
	("soppi", "noun"),
	("syli", "noun"),
	("säkä", "noun"),
	("tavata", "verb"),
	("tutti", "noun"),
	("tyvetä", "verb"),
	("vakaus", "noun"),
	("veto", "noun"),
	("viini", "noun"),
	("vika", "noun"),
	("vuori", "noun"),
}

generator_fst = kfst.FST.from_kfst_file(GENERATOR_FST_PATH)

POS_MORPHTAG_PATTERNS = {
	pos: list(read_list(os.path.join(CURR, 'patterns', f'pos-{pos}-patterns.txt'))) or ['']
	for pos in POS_FST_SOURCES
}

def generate_inflection_paradigm(word: str, pos: str, homonym: str = ''):

	"""
	Return a mapping of morphological tags to worforms.
	"""

	if is_uninflectable(word):
		return set()
	if not is_valid_pos(pos):
		return set()

	inflections = {}
	for source in POS_FST_SOURCES[pos]:
		for morphtags in POS_MORPHTAG_PATTERNS[pos]:
			forms = generate_wordform(word, pos, morphtags, homonym, source)
			if forms:
				inflections[morphtags] = list(forms)
		if inflections:
			break
	return inflections


def generate_forms(word: str, pos: str | None = None, homonym: str = ''):

	"""
	Return default set of unannotated standard inflected forms for given word (lemma).
	"""

	if is_uninflectable(word):
		return set()
	if not is_valid_pos(pos):
		return set()

	# Return all valid interpretations if POS tag has not been specified
	if not pos:
		return {form for pos in pos_tag(word) for form in generate_forms(word, pos, homonym)}

	for source in POS_FST_SOURCES[pos]:
		forms = set()
		for morphtags in POS_MORPHTAG_PATTERNS[pos]:
			forms.update(generate_wordform(word, pos, morphtags, homonym, source))
		if forms:
			return forms
	return set()


def generate_wordform(word: str, pos: str, morphtags: str, homonym: str = '', source: str ='Lexicon'):

	"""
	Generate set of valid inflected form specified by the morphological tags for the given word (lemma).
	"""

	if is_uninflectable(word):
		return set()
	if not is_valid_pos(pos):
		return set()

	# TODO: Make this work for other sources as well?
	if not homonym and (word, pos) in HOMONYMOUS and source == 'Lexicon':
		forms1 = generate_wordform(word, pos, morphtags, '1', source)
		forms2 = generate_wordform(word, pos, morphtags, '2', source)
		return forms1 | forms2

	forms = set()
	for word in add_compound_separators(word, pos=pos, normalize_separators=False):
		input_fields = source, word, f'^{pos}', str(homonym), '', morphtags
		input_string = TAB.join(input_fields)
		best = inf
		for form, weight in generator_fst.lookup(input_string):
			if weight > best:
				break
			forms.add(form)
			best = weight
	return forms
