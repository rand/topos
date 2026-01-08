module.exports = grammar({
  name: 'topos',

  extras: $ => [
    /[ \t\r]+/, // Horiz whitespace only
    $.comment,
  ],

  externals: $ => [
    $._indent,
    $._dedent,
    $._newline,
    $.prose,
  ],

  conflicts: $ => [
    [$.section],
    [$.requirement],
    [$.task],
    [$.subsection],
  ],

  rules: {
    source_file: $ => repeat($._item),

    _item: $ => choice(
      $.spec_def,
      $.import_def,
      $.section,
      prec(-1, $.prose),
      $._newline
    ),

    spec_def: $ => seq('spec', $.identifier, $._newline),

    import_def: $ => seq(
      'import',
      choice(
        seq(optional(seq('from', $.string)), ':', $.import_list),
        seq($.string, 'as', $.identifier)
      ),
      $._newline
    ),

    import_list: $ => seq(
      $.import_item,
      repeat(seq(',', $.import_item))
    ),

    import_item: $ => seq(
      $.reference,
      optional(seq('as', $.identifier))
    ),

    section: $ => seq(
      $.header,
      $._newline,
      repeat($._section_content)
    ),

    // Only single # for section headers; ## is reserved for requirements/tasks
    header: $ => seq(
      '#',
      $.prose
    ),

    _section_content: $ => choice(
      $.requirement,
      $.concept,
      $.task,
      $.behavior,
      $.invariant,
      $.aesthetic,
      $.foreign_block,
      $.subsection,
      $.prose,
      $._newline
    ),

    // Subsection headers (## Title) that aren't requirements or tasks
    subsection: $ => seq(
      '##',
      $.prose,
      $._newline,
      repeat(prec(1, choice($.prose, $._newline)))
    ),

    requirement: $ => seq(
      '##',
      alias($.req_id, $.identifier),
      ':',
      $.prose,
      $._newline,
      repeat(prec(1, choice($.ears_clause, $.acceptance, $.prose, $._newline)))
    ),

    req_id: $ => /REQ-[A-Z0-9-]+/,

    ears_clause: $ => seq(
      'when:', $.prose, $._newline,
      'the system shall:', $.prose, $._newline
    ),

    acceptance: $ => seq(
      'acceptance:', $._newline,
      $._indent,
      repeat1($.acc_clause),
      $._dedent
    ),

    acc_clause: $ => seq(
      choice('given:', 'when:', 'then:'),
      $.prose,
      $._newline
    ),

    concept: $ => seq(
      'Concept',
      $.identifier,
      ':',
      $._newline,
      $._indent,
      repeat1(choice($.field, $.prose, $._newline)),
      $._dedent
    ),

    field: $ => seq(
      'field',
      $.identifier,
      optional(seq('(', $.type_expr, ')')),
      optional(seq(':', repeat1($.constraint))),
      $._newline
    ),

    type_expr: $ => choice(
      $.reference,
      $.hole,
      seq('List', 'of', $.reference),
      seq('Optional', $.reference),                      // Optional `Type`
      seq($.reference, $.reference),                     // `Optional` `Type` (backtick form)
      seq('one', 'of:', $.variant_list)
    ),

    hole: $ => seq(
      '[?',
      optional($.hole_content),
      ']'
    ),

    hole_content: $ => /[^\]]+/,

    variant_list: $ => seq(
      $.identifier,
      repeat(seq(',', $.identifier))
    ),

    constraint: $ => choice(
      'unique',
      seq('default:', $.prose),
      seq('derived:', $.prose),
      seq('invariant:', $.prose),
      seq('at least', $.number, optional($.identifier))  // "at least 1 character"
    ),

    behavior: $ => seq(
      'Behavior', $.identifier, ':',
      $._newline,
      $._indent,
      repeat1(choice(
        $.implements_clause,
        $.behavior_body,
        $.prose,
        $._newline
      )),
      $._dedent
    ),

    implements_clause: $ => seq(
      'Implements', $.req_id, repeat(seq(',', $.req_id)), '.', $._newline
    ),

    behavior_body: $ => choice(
      seq('given:', $.prose, $._newline),
      seq('returns:', $.prose, $._newline),
      seq('requires:', $.prose, $._newline),
      seq('ensures:', $.prose, $._newline),
      $.ears_clause
    ),

    invariant: $ => seq(
      'Invariant', $.identifier, ':',
      $._newline,
      $._indent,
      repeat1(choice(
        $.quantifier,
        $.prose,
        $._newline
      )),
      $._dedent
    ),

    quantifier: $ => seq(
      'for each', $.identifier, 'in', $.reference, ':', $._newline
    ),

    task: $ => seq(
      '##',
      alias($.task_id, $.identifier),
      ':',
      $.prose,
      optional($.task_ref_list),  // [REQ-1, REQ-2] on same line
      $._newline,
      repeat(prec(1, choice(
        $.task_field,
        $.prose,
        $._newline
      )))
    ),

    task_id: $ => /TASK-[A-Z0-9-]+/,

    task_ref_list: $ => seq('[', alias($.req_id, $.identifier), repeat(seq(',', alias($.req_id, $.identifier))), ']'),

    task_field: $ => seq(
      choice('file:', 'tests:', 'depends:', 'status:', 'evidence:', 'context:'),
      $.prose,
      $._newline
    ),

    aesthetic: $ => seq(
      'Aesthetic', $.identifier, ':',
      $._newline,
      $._indent,
      repeat1(choice(
        $.aesthetic_field,
        $.prose,
        $._newline
      )),
      $._dedent
    ),

    aesthetic_field: $ => seq(
      $.identifier, ':', optional('[~]'), $.prose, $._newline
    ),

    foreign_block: $ => seq(
      '```',
      alias(/[a-z]+/, $.language),
      $._newline,
      repeat(seq($.prose, $._newline)),
      '```',
      $._newline
    ),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    reference: $ => seq('`', alias(/[^`]+/, $.identifier), '`'),

    string: $ => /"[^"]*"/,

    number: $ => /\d+/,

    comment: $ => token(seq('//', /.*/))
  }
});