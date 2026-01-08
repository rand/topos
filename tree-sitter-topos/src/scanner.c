#include <tree_sitter/parser.h>
#include <ctype.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

enum TokenType {
  INDENT,
  DEDENT,
  NEWLINE,
  PROSE,
};

typedef struct {
  uint16_t indents[64];
  uint8_t count;
} Scanner;

void *tree_sitter_topos_external_scanner_create() {
  Scanner *scanner = (Scanner *)calloc(1, sizeof(Scanner));
  scanner->indents[0] = 0;
  scanner->count = 1;
  return scanner;
}

void tree_sitter_topos_external_scanner_destroy(void *payload) {
  free(payload);
}

unsigned tree_sitter_topos_external_scanner_serialize(void *payload, char *buffer) {
  Scanner *scanner = (Scanner *)payload;
  unsigned size = 0;
  for (unsigned i = 0; i < scanner->count && size < 64; i++) {
    buffer[size++] = (char)scanner->indents[i];
  }
  return size;
}

void tree_sitter_topos_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
  Scanner *scanner = (Scanner *)payload;
  scanner->count = 0;
  if (length > 0) {
    for (unsigned i = 0; i < length && i < 64; i++) {
      scanner->indents[scanner->count++] = (uint16_t)buffer[i];
    }
  } else {
    scanner->indents[scanner->count++] = 0;
  }
}

static void skip(TSLexer *lexer) {
  lexer->advance(lexer, true);
}

static void advance(TSLexer *lexer) {
  lexer->advance(lexer, false);
}

static bool is_keyword(const char *word) {
    if (strcmp(word, "when:") == 0) return true;
    if (strcmp(word, "given:") == 0) return true;
    if (strcmp(word, "then:") == 0) return true;
    if (strcmp(word, "acceptance:") == 0) return true;
    if (strcmp(word, "returns:") == 0) return true;
    if (strcmp(word, "requires:") == 0) return true;
    if (strcmp(word, "ensures:") == 0) return true;
    if (strcmp(word, "Concept") == 0) return true;
    if (strcmp(word, "Behavior") == 0) return true;
    if (strcmp(word, "Invariant") == 0) return true;
    if (strcmp(word, "Aesthetic") == 0) return true;
    if (strcmp(word, "field") == 0) return true;
    if (strcmp(word, "spec") == 0) return true;
    if (strcmp(word, "import") == 0) return true;
    if (strcmp(word, "from") == 0) return true;
    if (strcmp(word, "the") == 0) return true; 
    if (strcmp(word, "system") == 0) return true; 
    if (strcmp(word, "shall:") == 0) return true; 
    if (strcmp(word, "Implements") == 0) return true;
    if (strcmp(word, "file:") == 0) return true;
    if (strcmp(word, "tests:") == 0) return true;
    if (strcmp(word, "status:") == 0) return true;
    if (strcmp(word, "evidence:") == 0) return true;
    if (strcmp(word, "context:") == 0) return true;
    if (strncmp(word, "##", 2) == 0) return true;
    if (word[0] == '#') return true;
    return false;
}

bool tree_sitter_topos_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  Scanner *scanner = (Scanner *)payload;

  // Horizontal whitespace skip
  while (lexer->lookahead == 32 || lexer->lookahead == 9 || lexer->lookahead == 13) {
    skip(lexer);
  }

  // 1. Newline
  if (valid_symbols[NEWLINE] && lexer->lookahead == 10) {
    advance(lexer);
    lexer->mark_end(lexer); // MARK
    lexer->result_symbol = NEWLINE;
    return true;
  }

  // 2. Prose (peek first word)
  if (valid_symbols[PROSE] && lexer->lookahead != 10 && lexer->lookahead != 0) {
    char word[64] = {0};
    int i = 0;
    
    // Check for special punctuation
    if (lexer->lookahead == '#' || lexer->lookahead == '`') {
        // We don't advance, we just return false
        return false; 
    }

    // Read word for keyword check
    // We need to 'peek' without consuming if we fail.
    // Tree-sitter resets state if we return false. So we can advance safely as long as we return false.
    
    int32_t current = lexer->lookahead;
    while (current != 0 && !isspace(current) && i < 63) {
        word[i++] = (char)current;
        advance(lexer);
        current = lexer->lookahead;
    }
    word[i] = '\0';
    
    if (is_keyword(word)) {
        return false; // Backtrack
    }
    
    // Not a keyword. We need to consume the REST of the line.
    // NOTE: We already consumed the first word. Continue.
    while (lexer->lookahead != 10 && lexer->lookahead != 0) {
        advance(lexer);
    }
    
    lexer->mark_end(lexer); // MARK
    lexer->result_symbol = PROSE;
    return true;
  }

  // 3. Indent/Dedent
  if (valid_symbols[INDENT] || valid_symbols[DEDENT]) {
    uint16_t current_indent = lexer->get_column(lexer);

    if (valid_symbols[INDENT] && current_indent > scanner->indents[scanner->count - 1]) {
      scanner->indents[scanner->count++] = current_indent;
      lexer->result_symbol = INDENT;
      return true;
    }

    if (valid_symbols[DEDENT] && current_indent < scanner->indents[scanner->count - 1]) {
      scanner->count--;
      lexer->result_symbol = DEDENT;
      return true;
    }
  }

  return false;
}