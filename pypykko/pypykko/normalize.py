#! /usr/bin/env python3

import sys
from collections import defaultdict
from .constants import SENT_BREAK, OPENING_TAGS

inf = float('inf')
indices = defaultdict(float)
LEADING_PUNCTUATION = set('-–—"”„’([')


def is_lowercase(w):
	return not is_uppercase(w)


def is_uppercase(w):
	return w[0:1] == w[0:1].upper()


def lettercase_match(w1, w2):
	return (is_lowercase(w1) and is_lowercase(w2)) or (is_uppercase(w1) and is_uppercase(w2))


def fix_lettercase(wform, lemma):
	if is_uppercase(wform) and is_lowercase(lemma):
		return lemma[0:1].upper() + lemma[1:]
	return lemma


def process_analyses(analyses, sentence_initial=None):

	best = inf, inf

	for analysis in analyses:

		wform, source, lemma, pos, hom, style, tags, weight = analysis
		weight = float(weight)

		# (*) Do not penalize capitalization sentence-initially
		if sentence_initial and is_uppercase(wform) and is_lowercase(lemma) and weight:
			weight -= 1

		# (*) Prioritize analyses where wordform and lemma lettercases match mid-sentence
		if not sentence_initial and not lettercase_match(wform, lemma):
			weight += 1

		# (*) No capitalized verbs mid-sentence
		if not sentence_initial and pos == 'verb' and is_uppercase(wform):
			weight += 1

		# (*) Penalize proper names with possessive suffixes
		# if pos in ['proper', 'proper-pl'] and is_uppercase(lemma) and '+poss' in tags:
		# 	weight += 0.5

		# (*) Penalize proper names in plural
		# if pos in ['proper'] and is_uppercase(lemma) and '+pl' in tags:
		# 	weight += 0.5

		# (*) Penalize instructive and comitative case
		# if '+ins' in tags or '+com' in tags:
		# 	weight += 0.5

		# if not sentence_initial:
		# 	lemma = fix_lettercase(wform, lemma)

		index = indices[lemma, pos] or inf
		pair = weight, index
		best = min(best, pair)

		# Reassign values
		analysis[0:] = wform, source, lemma, pos, hom, style, tags, weight, index

		# print(analysis)

	analyses.sort(key=lambda a: a[-1])
	filtered = []

	for a in analyses:
		_, _, _, _, _, _, _, weight, index = a
		if (weight, index) == best:
			print('%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s' % tuple(a[:8]))
			filtered.append(tuple(a[:8]))
	print()
	return filtered


def main():

	analyses = []
	analysis = '', '', '', '', ''
	sentence_initial = True

	for line in sys.stdin:

		line = line.strip('\n\r')

		if not line and analyses:
			process_analyses(analyses, sentence_initial)
			prev_wform, _, _, _, _, _, _, _, _ = analysis
			sentence_initial = (
				sentence_initial if prev_wform in LEADING_PUNCTUATION else
				prev_wform in [SENT_BREAK] + OPENING_TAGS
			)
			analyses.clear()
			continue

		analysis = line.split('\t')
		analyses.append(analysis)


if __name__ == '__main__':
	main()
