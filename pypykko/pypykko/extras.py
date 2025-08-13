from .utils import PARSER_FST, compare_with_others, inf

def analyze_with_compound_parts(word, only_best=True, normalize_separators=True, ignore_derivatives=True):
	"""
	Return list of tuples (morphological analyses) with duplicates removed.
	Only best analyses are returned by default.
	A final tuple component of compound ranges in the original uninflected word is returned.
	"""

	analyses = []
	final_ranges = []
	taken = {}
	input_pieces = PARSER_FST.split_to_symbols(word)
	assert input_pieces is not None
	for raw_analysis, weight in list(PARSER_FST.lookup_aligned(word)):
		raw_analysis = list(raw_analysis)
		piece_ranges = []
		for output_piece_idx, (input_piece_idx, piece) in enumerate(raw_analysis):
			if output_piece_idx < 2:
				continue
			if "\t" in piece.get_symbol():
				if len(piece_ranges) == 0:
					piece_ranges.append(range(0, len(input_pieces)))
				else:
					piece_ranges.append(range(piece_ranges[-1].stop, len(input_pieces)))
				break
			if piece.get_symbol() in ("|", "⁅BOUNDARY⁆", "⁅HYPHEN⁆"):
				if len(piece_ranges) == 0:
					piece_ranges.append(range(0, input_piece_idx))
				else:
					piece_ranges.append(range(piece_ranges[-1].stop, input_piece_idx))
		char_idxs = [0]
		for piece in input_pieces:
			char_idxs.append(char_idxs[-1] + len(piece.get_symbol()) if not piece.is_epsilon() else 0)
		
		char_ranges = []
		for r in piece_ranges:
			char_ranges.append(range(char_idxs[r.start], char_idxs[r.stop]))
		char_ranges = tuple(char_ranges)
		
		analysis_string = "".join(x[1].get_symbol() for x in raw_analysis if not x[1].is_epsilon())

		if normalize_separators:
			analysis_string = analysis_string.replace('⁅BOUNDARY⁆', '|').replace('⁅HYPHEN⁆', '-')

		if taken.get((analysis_string, char_ranges)):
			continue

		taken[(analysis_string, char_ranges)] = True
		analysis = [word] + analysis_string.split('\t') + [weight]
		analyses.append(analysis)
		final_ranges.append(char_ranges)

	best = inf
	filtered = []
	for analysis, char_ranges in zip(analyses, final_ranges):

		_, _, _, _, _, _, _, weight = analysis

		if only_best and weight > best:
			break

		response = compare_with_others(analysis, analyses)

		if response == 'has-derivative' and ignore_derivatives:
			continue

		filtered.append(tuple(analysis) + (char_ranges,))
		best = weight

	return filtered
