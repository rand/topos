; Topos Language Highlight Queries
; Tree-sitter highlighting for .tps/.topos files

; ===========================================================================
; Keywords - Language constructs
; ===========================================================================

"spec" @keyword
"import" @keyword.import
"from" @keyword.import
"as" @keyword

; Block keywords
"Concept" @keyword.type
"Behavior" @keyword.function
"Invariant" @keyword.type
"Aesthetic" @keyword.type
"Implements" @keyword

; Field and type keywords
"field" @keyword
"List" @keyword.type
"of" @keyword
"Optional" @keyword.type
"one" @keyword
"unique" @keyword.modifier

; Clause keywords (BDD/EARS)
"when:" @keyword.conditional
"then:" @keyword.conditional
"given:" @keyword.conditional
"the system shall:" @keyword.conditional
"acceptance:" @keyword

; Behavior body keywords
"returns:" @keyword.return
"requires:" @keyword.conditional
"ensures:" @keyword.conditional

; Constraint keywords
"default:" @keyword
"derived:" @keyword
"invariant:" @keyword
"at least" @keyword

; Task field keywords
"file:" @keyword
"tests:" @keyword
"depends:" @keyword
"status:" @keyword
"evidence:" @keyword
"context:" @keyword

; Invariant keywords
"for each" @keyword.repeat
"in" @keyword

; ===========================================================================
; Identifiers and References - Context-based highlighting
; ===========================================================================

; Requirement ID (first identifier in requirement block)
(requirement
  (identifier) @constant)

; Task ID (first identifier in task block)
(task
  (identifier) @constant)

; Requirement references in task ref list
(task_ref_list
  (identifier) @constant)

; Type references in backticks
(reference
  (identifier) @type)

; Concept name after keyword
(concept
  (identifier) @type.definition)

; Behavior name after keyword
(behavior
  (identifier) @function)

; Invariant name after keyword
(invariant
  (identifier) @type.definition)

; Aesthetic name after keyword
(aesthetic
  (identifier) @type.definition)

; Field name
(field
  (identifier) @property)

; Spec name
(spec_def
  (identifier) @module)

; Import alias
(import_item
  (identifier) @variable)

; Variant list items
(variant_list
  (identifier) @constant)

; General identifiers (fallback)
(identifier) @variable

; ===========================================================================
; Typed Holes
; ===========================================================================

(hole
  "[?" @punctuation.special
  "]" @punctuation.special)

(hole_content) @comment

; ===========================================================================
; Soft Constraints
; ===========================================================================

"[~]" @punctuation.special

; ===========================================================================
; Strings and Prose
; ===========================================================================

(string) @string
(prose) @string.special

; ===========================================================================
; Numbers
; ===========================================================================

(number) @number

; ===========================================================================
; Headers
; ===========================================================================

(header
  "#" @markup.heading)

(requirement
  "##" @markup.heading)

(task
  "##" @markup.heading)

(subsection
  "##" @markup.heading)

; ===========================================================================
; Foreign Blocks (TypeSpec, CUE)
; ===========================================================================

(foreign_block
  "```" @punctuation.delimiter)

(foreign_block
  (language) @label)

; ===========================================================================
; Comments
; ===========================================================================

(comment) @comment

; ===========================================================================
; Punctuation
; ===========================================================================

":" @punctuation.delimiter
"," @punctuation.delimiter
"." @punctuation.delimiter
"[" @punctuation.bracket
"]" @punctuation.bracket
"(" @punctuation.bracket
")" @punctuation.bracket
"`" @punctuation.special
