import re
import sys

from .file_tools import load_json
from .constants import POS_TAGS
from collections import defaultdict

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

def validate_pos(pos):
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


def determine_wordform_harmony(wordform, default=None):
	if default:
		return default.upper()
	for c in reversed(wordform.lower()):
		if c in set('y'):
			return 'FRONT'
		if c in set('aouáóúàòùâôû'):
			return 'BACK'
		if c in set('äöüø'):
			return 'FRONT'
		if c in set('14579'):
			return 'FRONT'
		if c in set('2368'):
			return 'BACK'
	return 'FRONT'


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