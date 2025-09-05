import pathlib
import os
scripts_path = pathlib.Path(__file__).parent.resolve()

ALPHA_UPPER_EXTENDED = \
	'ABCDEFGHIJKLMNOPQRSTUVWXYZ' \
	'ÁÉÍÓÚÝ' \
	'ÀÈÌÒÙỲ' \
	'ÂÊÎÔÛŶĈĜĴĤ' \
	'ÄËÏÖÜŸ' \
	'ĀĒĪŌŪȲ' \
	'ŠŽČŇĽŘŤĎĚ' \
	'ŚŹĆŃĹŔ' \
	'ÃÕÑ' \
	'ÅŮ' \
	'ŞÇ' \
	'ĄĘŲĢ' \
	'ĖŻ' \
	'ȘŅĻŖȚĶ' \
	'ØßÐĐÆŒŁĞŐŊÞ'

ALPHA_UPPER = 'ABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖÜŠŽČĆ'
ALPHA_LOWER = 'abcdefghijklmnopqrstuvwxyzåäöüšžčćı'

PARSER_FST_PATH = os.path.join(scripts_path, 'fi-parser.kfst')
GENERATOR_FST_PATH = os.path.join(scripts_path, 'fi-generator.kfst')

LINE_BREAK = '@_LINEBREAK_@'
SENT_BREAK = '@_SENTBREAK_@'
ZERO = '@_zero_@'
TAB = '^TAB'

OPENING_TAGS = [f'<{tag}>' for tag in ['h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'p']]

STYLE_TAGS = [
	'+arch',
	'+child',
	'+coll',
	'+dated',
	'+dial',
	'+foreign',
	'+jocul',
	'+nstd',
	'+poet',
	'+rare',
	'+slang',
	'+vulg',
]
STYLE_TAG_REGEX = '|'.join(tag[1:] for tag in STYLE_TAGS)

POS_TAGS = [
	'noun',
	'noun-pl',
	'verb',
	'adverb',
	'ordinal',
	'numeral',
	'pronoun',
	'pronoun-pl',
	'adjective',
	'adposition',
	'conjunction',
	'interjection',
	'adverb+verb',
	'conjunction+verb',
	'proper',
	'proper-pl',
	'participle',
	'none',
]

CLITICS = {
"+han",
"+ka",
"+kaan",
"+kin",
"+ko",
"+pa",
"+poss1pl",
"+poss1sg",
"+poss2pl",
"+poss2sg",
"+poss3",
"+poss3",
"+s",
}

FIELDS = [
	'',    # 1. source
	'%s',  # 2. lemma
	'',    # 3. pos
	'',    # 4. homonym
	'',    # 5. style
	''     # 6. morph.tags
]

FIELD_STRING = '\t'.join(FIELDS)
