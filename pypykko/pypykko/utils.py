import re
from .constants import PARSER_FST_PATH, FIELD_STRING
import kfst
from .scriptutils import validate_pos

C = "[bcdfghjklmnpqrstvwxzšžč'’]"
V = '[aeiouyäöüå]'

PARSER_FST = kfst.FST.from_kfst_file(PARSER_FST_PATH)

inf = float('inf')


def unk_result(wordform):
	return FIELD_STRING % wordform


def remove_separators(lemma, wordform=None):

	# TODO: Actually do this in a way that takes vertical lines in wordform into consideration.

	return lemma.replace('|', '').replace('⁅HYPHEN⁆', '-').replace('⁅BOUNDARY⁆', '')


def lookup(wordform):
	return PARSER_FST.lookup(wordform)


def is_derived_from(analysis_a, analysis_b):

	_, _, _, _, _, _, morphtags_a, _ = analysis_a
	_, _, _, _, _, _, morphtags_b, _ = analysis_b

	if not morphtags_a and morphtags_b:
		return False

	return morphtags_b.startswith('+deriv_') and morphtags_b.endswith(morphtags_a)


def is_participle_of(analysis_a, analysis_b):

	_, _, _, pos_a, _, _, morphtags_a, _ = analysis_a
	_, _, _, pos_b, _, _, morphtags_b, _ = analysis_b

	if not morphtags_a and morphtags_b:
		return False

	return pos_a == 'participle' and morphtags_b.startswith('+part_') and morphtags_b.endswith(morphtags_a)


def compare_with_others(a_source, analyses):

	_, _, lemma_source, pos, _, _, morphtags_source, _ = a_source

	if pos != 'verb':
		return

	for a_target in analyses:

		if a_target == a_source:
			continue

		_, _, lemma_target, _, _, info, morphtags_target, _ = a_target

		if ' ← ' in info:
			return

		if is_derived_from(a_target, a_source):
			deriv_tag = morphtags_source.replace(morphtags_target, '')
			a_target[5] += f' ← {pos}:{lemma_source}:{deriv_tag}'
			return 'has-derivative'

		if is_participle_of(a_target, a_source):
			participle_tag = morphtags_source.replace(morphtags_target, '')
			a_target[5] += f' ← {pos}:{lemma_source}:{participle_tag}'
			return 'has-participle'


def analyze(word, only_best=True, normalize_separators=True, ignore_derivatives=True):

	"""
	Return list of tuples (morphological analyses) with duplicates removed.
	Only best analyses are returned by default.
	"""

	analyses = []
	taken = {}
	for analysis_string, weight in list(PARSER_FST.lookup(word)) or [(unk_result(word), inf)]:

		if normalize_separators:
			analysis_string = analysis_string.replace('⁅BOUNDARY⁆', '|').replace('⁅HYPHEN⁆', '-')

		if taken.get(analysis_string):
			continue

		taken[analysis_string] = True
		analysis = [word] + analysis_string.split('\t') + [weight]
		analyses.append(analysis)

	best = inf
	filtered = []
	for analysis in analyses:

		_, _, _, _, _, _, _, weight = analysis

		if only_best and weight > best:
			break

		response = compare_with_others(analysis, analyses)

		if response == 'has-derivative' and ignore_derivatives:
			continue

		filtered.append(analysis)
		best = weight

	return [tuple(a) for a in filtered]


def add_compound_separators(word, pos=None, normalize_separators=True, pick_first=False):

	# TODO: Allow adding separators to non-lemma words?

	valid = set()
	best = inf
	for a in analyze(word, only_best=False, normalize_separators=normalize_separators):
		_, _, lemma, p, _, _, _, weight = a
		if pos and p != pos:
			continue
		if weight > best:
			break
		if remove_separators(lemma) == word:
			valid.add(lemma)
			best = weight
	if pick_first:
		return sorted(valid or {word})[0]
	return valid or {word}


def is_plural(word):
	for _, _, lemma, pos, _, _, morphtags, weight in analyze(word, only_best=True):
		if morphtags == '+pl+nom':
			return lemma
		if pos == 'noun-pl' and morphtags == '+nom':
			return lemma
	return False

def singularize(word):
	return is_plural(word) or word

def pos_tag(word, force_match=False, max_weight=inf):

	if force_match:
		tags = set()
		best_weight = max_weight
		for _, _, w, pos, _, _, _, weight in analyze(word, only_best=False):
			if weight == inf or weight > best_weight:
				break
			if remove_separators(w) == word and pos:
				tags.add(pos)
				best_weight = weight
		return tags

	return set(
		pos for _, _, w, pos, _, _, _, weight in analyze(word, only_best=True) if remove_separators(w) == word and pos
		if weight <= max_weight
	)


def lemmatize(word, pos=None):
	valid = set()
	for a in analyze(word, only_best=True):
		_, _, w, p, _, _, _, _ = a
		if pos and p != pos:
			continue
		valid.add(w)
	return valid


def syllabify(word, pos=None, compound=True):

	validate_pos(pos)
	word = add_compound_separators(word, pos, pick_first=True) if compound else word

	# ma·ya
	word = re.sub(f'(?<=[aeiou])(?=y[aeou])', '·', word)

	# ikty·ologi, viipy·ä
	word = re.sub(f'(?<=[a-zåäö]y)(?=[äo])', '·', word)

	# make·a
	word = re.sub(f'(?<=[eiouö])(?=[aä])', '·', word)

	# selvi·ö
	word = re.sub(f'(?<=[aeiouä])(?=ö)', '·', word)

	# alki·o
	word = re.sub(f'(?<=[aeiäö])(?=o)', '·', word)

	# ko·e
	word = re.sub(f'(?<=[aouäö])(?=e)', '·', word)

	# kan·si, kant·ti, angs·ti, halst·rata
	word = re.sub(f'(?<={V})({C}+)(?={C}{V})', r'\1·', word)
	word = re.sub(f'(?<={V})({C}+)(?={C}{V})', r'\1·', word)

	# ka·la
	word = re.sub(f'(?<={V})(?={C}{V})', '·', word)

	# kofe·ii, Mari·aanit
	word = re.sub(f'(?<={V})(?=aa|ee|ii|oo|uu|yy|ää|öö)', '·', word)

	# kau·an, liu·os
	word = re.sub(f'(?<=[aeiou][iu])(?={V})', '·', word)

	# nei·yt
	word = re.sub(f'(?<=[äeiöy][iy])(?={V})', '·', word)

	# ruo·an
	word = re.sub(f'(?<=ie|uo|yö)(?={V})', '·', word)

	# raa·istua
	word = re.sub(f'(?<=aa|ee|ii|oo|uu|yy|ää|öö)(?={V})', '·', word)

	# cesi·um
	word = re.sub(f'(?<=[ei])(?=um)', '·', word)

	return word


def add_compound_separators_to_proper_name(name):

	def restore_letter_case(s, n):
		segments = []
		for part in s.split('|'):
			segment, n = n[:len(part)], n[len(part):]
			segments.append(segment)
		return '|'.join(segments)

	for pos in ['proper-pl', 'proper']:
		separated = add_compound_separators(name, pos)
		if separated != {name}:
			return separated

	word = name.lower()
	for pos in ['noun-pl', 'noun']:
		separated = add_compound_separators(word, pos)
		if separated != {word}:
			separated = {restore_letter_case(s, name) for s in separated}
			return separated

	return {name}


def transfer_separators(source, target):
	segments = []
	for part in source.split('|')[:-1]:
		if target.lower().startswith(part.lower()):
			segments.append(target[:len(part)])
			target = target[len(part):]
		else:
			break
	segments.append(target)
	return '|'.join(segments)
