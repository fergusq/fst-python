import re
import sys

from .file_tools import load_json
from .constants import POS_TAGS
from collections import defaultdict
from .utils import syllabify
C = '[bcdfghjklmnpqrstvwxz]'

try:
	ADVERB_INFLECTIONS = load_json(filename='adverbs.json', directory='scripts/inflection')
except FileNotFoundError:
	ADVERB_INFLECTIONS = {}

INTERROGATIVES = [
	'kuka',
	'mikä',
	'ken',
	'missä',
	'mistä',
	'mihin',
	'minne',
	'miksi',
	'mitä',
	'miten',
	'milloin',
	'koska',
	'kuinka',
]


def is_valid_pos(pos):
	if pos and pos not in POS_TAGS:
		print(sys.stderr.write(f'Warning! Unknown POS tag "{pos}"\n'))
		return False
	return True


def get_wordform(pairs):
	return ''.join(c for _, c in pairs if c != '0')


def get_input_string(pairs):
	return ''.join(c for c, _ in pairs)


def get_output_string(pairs):
	return ''.join(c for _, c in pairs)


def get_input_and_output_strings(pairs):
	return get_input_string(pairs), get_output_string(pairs)


def get_lemma_length(inflections):
	[lemma] = inflections['@base']
	return len(lemma)


def get_morphtag_characters(s: str):
	"""
	"sg|nom" => ["+sg", "+nom"]
	"""
	return [f'+{tag}' for tag in s.split('|') if tag and not tag.startswith('@')]


def get_tags(morphtag: str):
	"""
	"sg|nom" => {"sg", "nom"}
	"""
	return set(morphtag.split('|'))


def has_agreement(lemma):
	return re.fullmatch('.+%.+', lemma)


def determine_separator(w1, w2, default='0', strip_zeros=True):

	w1 = w1.strip('0') if strip_zeros else w1
	w2 = w2.strip('0') if strip_zeros else w2

	if w1.startswith('-'):
		return ''

	c1 = w1[-1:]
	c2 = w2[:1]
	if c1 == c2 and c2 in set('aeiouyäö'):
		return '-'
	return default


def get_parts(lemma):
	return re.findall(r'[^-|% ]+[-|% ]?', lemma) or [lemma]


def get_base_lemma(lemma):
	return get_parts(lemma)[-1]


def count_syllables(lemma):
	syllabified = syllabify(lemma, compound=False)
	return len(syllabified.split('·'))


def determine_lemma_vowel_harmony(lemma, kotus_class=None):

	lemma = get_parts(lemma).pop()

	# "onomatopoeettinen"
	if re.fullmatch('.*(poeettinen)', lemma):
		return 'back'

	# "prototyyppi", "prototyyppinen", "geotekninen", "biokteknisesti"
	if re.fullmatch('.*('
		'depressiivi|elementti|elementtisesti|kineettinen|kineettisesti|kliininen|kliinisesti|oeettinen|oeettisesti|semiitti|semiittinen|semitismi|semitisti|semitistinen|semitistisesti|sentrinen|sentrisesti|sentrismi|synteesi|synteettinen|synteettisesti|tekninen|teknisesti|tyyppi|tyyppinen|tyyppisesti|syklinen|'
		'syklisesti|psyykkinen|psyykkisesti|fyysinen|fyysisesti)', lemma):
		return 'front'

	# "makromolekyyli", "psykoanalyyttinen"
	if re.fullmatch('.*(aldehydi|analyysi|analyyttinen|analyyttisesti|molekyyli|molekyylinen)', lemma):
		return 'front|back'

	# "porfyyri", polyyppi", "dialyysi", "porfyriini", "molybdeeni"
	if re.fullmatch(f'.*[aou].*(y{C}{C}?i|y{C}{C}?inen|y{C}{C}?isesti|y{C}{C}?ismi|y{C}{C}?isti|y{C}{C}?ii{C}{C}?i|y{C}{C}?ee{C}{C}?i)', lemma):
		return 'front|back'

	#  "anglofiili", "karsinogeeni", "telomeeri", "ortopedi", "antisepti", "dynamometri"/"barometri" "hypoteesi"
	if count_syllables(lemma) >= 4 and re.fullmatch('.*[aou].*(geeni|iili|meeri|metri|pedi|septi|teesi)', lemma):
		return 'front|back'

	# "fylogeneesi", "fylogeneettisesti"
	if re.fullmatch('.*[aou].*(elektrinen|elektisesti|fiili|fiilinen|fiilisesti|geeninen|geenisesti|geneesi|geneettinen|geneettisesti|metrinen|metrisesti|pedinen|pedisesti|septinen|septisesti|teismi|teisti|teistinen|teistisesti|terminen|termisesti|tsepiini)', lemma):
		return 'front|back'

	# Initialisms and numbers
	if re.fullmatch('.*[14579BCDEFGIJLMNPRSTVWXYÄÖÜÉ]', lemma):
		return 'front'
	if re.fullmatch('.*[2368AHKOQUZÅ]', lemma):
		return 'back'
	if re.fullmatch('.*[123456789]0(:s)?', lemma):
		return 'front'
	if re.fullmatch('.+oy', lemma):
		return 'back'
	if re.fullmatch(f'.*[aouAOU]{C}+y', lemma):
		return 'back'
	if re.fullmatch('.*[aouAOU].*y', lemma):
		return 'front|back'
	if kotus_class in {'18B', '10B'} and lemma[-1] in set('bcdefgijlmnprstvwxyzäöüé'):
		return 'front'
	if kotus_class in {'18B', '10B'}:
		return 'back'

	return determine_wordform_harmony(lemma)


def determine_wordform_harmony(wordform, default_harmony=None):
	if default_harmony in {'front', 'back'}:
		return default_harmony
	for c in reversed(wordform.lower()):
		if c in set('y'):
			return 'front'
		if c in set('aouáóúàòùâôûå'):
			return 'back'
		if c in set('äöüø'):
			return 'front'
		if c in set('14579'):
			return 'front'
		if c in set('2368'):
			return 'back'
	return 'front'


def unpack(classes='', gradations='', harmonies='', vowels='', ignore_styles=False):

	classes = classes.replace('?', '').replace('!', '')

	if ignore_styles:
		classes = classes.replace('†', '').replace('‡', '').replace(')', '').replace('(', '')
		gradations = gradations.replace('†', '').replace('‡', '').replace(')', '').replace('(', '')

	classes = [classes] if re.findall('[†‡)(]', classes) else classes.split('|')
	gradations = [gradations] if re.findall('[†‡)(]', gradations) else gradations.split('|')
	gradations = ['' if grad == '=' else grad for grad in gradations]
	harmonies = harmonies.split('|')
	vowels = vowels.split('|')

	return [
		(c, g, h, v)
		for c in classes
		for g in gradations
		for h in harmonies
		for v in vowels
	]


def uniqlist(l: list):
	return sorted(set(l), key=lambda x: l.index(x))


def clean(inflections: dict):
	return {key: uniqlist(val) for key, val in inflections.items()}


def ddict(d: dict):
	result = defaultdict(list)
	result.update(d)
	return result

def is_uninflectable(lemma):

	"""
	Return True if string is or ends with punctuation.
	"""

	return not lemma or lemma[-1] in set('.:;-')


"""
def combine(obj1: dict, obj2: dict):
	keys = set(obj1.keys()) | set(obj2.keys())
	combined = {}
	for key in keys:
		combined[key] = obj1.get(key, []) + obj2.get(key, [])
	return clean(combined)


def combine_objs(objs):
	combined = {}
	for obj in objs:
		combined = combine(combined, obj)
	return combined
"""
