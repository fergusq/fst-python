from .constants import GENERATOR_FST_PATH, TAB
from .utils import add_compound_separators, inf
import kfst


generator_fst = kfst.FST.from_kfst_file(GENERATOR_FST_PATH)


def generate_wordform(word: str, pos: str, morphtags: str, homonym: str = '', source='Lexicon'):

	word = add_compound_separators(word, pos=pos, normalize_separators=False, pick_first=True)

	input_fields = source, word, f'^{pos}', str(homonym), '', morphtags
	input_string = TAB.join(input_fields)

	forms = set()
	best = inf
	for form, weight in generator_fst.lookup(input_string):
		if weight > best:
			break
		forms.add(form)
		best = weight
	return forms


if __name__ == '__main__':
	print(generate_wordform('suuri', 'adjective', '+sg+gen'))
	print(generate_wordform('kissakoira', 'noun', '+pl+par', source='Lexicon|Pfx'))
	print(generate_wordform('-rakenteinen', 'adjective', '+sg+ine', source='Lexicon|Hyp'))