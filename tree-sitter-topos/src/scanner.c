#include <tree_sitter/parser.h>
#include <ctype.h>
#include <string.h>

enum TokenType {
  PROSE,
};

void *tree_sitter_topos_external_scanner_create() {
  return NULL;
}

void tree_sitter_topos_external_scanner_destroy(void *payload) {}

unsigned tree_sitter_topos_external_scanner_serialize(void *payload, char *buffer) {
  return 0;
}

void tree_sitter_topos_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {}

static bool is_keyword(const char *word) {
    const char *keywords[] = {
        "spec", "import", "Concept", "Behavior", "Invariant", "Aesthetic", "field",
        "when:", "given:", "then:", "requires:", "ensures:", "acceptance:",
        "the", // for "the system shall:"
        "Implements", "file:", "tests:", "depends:", "status:", "evidence:", "context:",
        "##",
        NULL
    };

    for (int i = 0; keywords[i]; i++) {
        if (strcmp(word, keywords[i]) == 0) {
            return true;
        }
        // Handle "the system shall:" prefix check if needed, but "the" covers it for first word
        // Handle "##"
        if (strcmp(keywords[i], "##") == 0 && strncmp(word, "##", 2) == 0) return true;
    }
    return false;
}

bool tree_sitter_topos_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  if (valid_symbols[PROSE]) {
    // Skip whitespace
    while (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
      lexer->advance(lexer, true);
    }

    if (lexer->lookahead == '\n' || lexer->lookahead == '\r' || lexer->lookahead == 0) {
      return false; // Empty line is not prose, let regular parser handle newline/whitespace
    }

    // Read first word to check if it is a keyword
    char first_word[64] = {0};
    int i = 0;
    
    lexer->mark_end(lexer); // Mark start of prose

    // Peek ahead to check keyword
    // We cannot consume yet if we want to reject.
    // Actually, if we reject, we just return false and consume nothing?
    // But scanner must consume token if it returns true.
    
    // We need to implement: If matches keyword -> return false.
    // If not keyword -> consume line -> return true.

    // Peek first word
    int32_t current = lexer->lookahead;
    while (current != 0 && !isspace(current) && i < 63) {
        first_word[i++] = (char)current;
        lexer->advance(lexer, false);
        current = lexer->lookahead;
    }
    first_word[i] = '\0';

    if (is_keyword(first_word)) {
        // It's a keyword, so we should NOT match PROSE.
        // We scanned some chars, but we return false, so they are not consumed?
        // Wait, TSLexer state is mutable.
        // We assume returning false resets the state? No, we must rely on not calling mark_end later?
        // Tree-sitter resets position if scan returns false? Yes.
        return false;
    }

    // Not a keyword. Consume the rest of the line.
    while (lexer->lookahead != '\n' && lexer->lookahead != 0) {
      lexer->advance(lexer, false);
    }
    
    lexer->mark_end(lexer);
    lexer->result_symbol = PROSE;
    return true;
  }

  return false;
}
