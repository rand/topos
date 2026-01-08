module.exports = grammar({
  name: 'topos',

  extras: $ => [
    /\s+/,
    $.comment,
  ],

  externals: $ => [
    $.prose,
  ],

  conflicts: $ => [
    [$.section, $.prose],
    [$._section_content, $.prose],
    [$.behavior, $.prose],
    [$.invariant, $.prose],
  ],

  rules: {
    source_file: $ => repeat($._item),

    _item: $ => choice(
      $.spec_def,
      $.import_def,
      $.section,
      $.prose
    ),

    spec_def: $ => seq('spec', $.identifier),

    import_def: $ => seq(
      'import',
      choice(
        seq(optional(seq('from', $.string)), ':', choice('*', $.import_list)),
        seq($.string, 'as', $.identifier)
      )
    ),

    import_list: $ => seq(
      $.import_item,
      repeat(seq(',', $.import_item))
    ),

    import_item: $ => seq(
      $.reference,
      optional(seq('as', $.identifier))
    ),

    section: $ => prec.left(seq(
      $.header,
      repeat($._section_content)
    )),

    header: $ => seq(
      token(repeat1('#')),
      token.immediate(/[^\n]+/)
    ),

    _section_content: $ => choice(
      $.requirement,
      $.concept,
      $.task,
      $.behavior,
      $.invariant,
      $.aesthetic,
      $.foreign_block,
      $.prose
    ),

    requirement: $ => prec.left(seq(
      '##',
      alias($.req_id, $.identifier),
      ':',
      token.immediate(/[^\n]+/),
      repeat(choice($.ears_clause, $.acceptance, $.prose))
    )),

    req_id: $ => /REQ-[A-Z0-9-]+/,

    ears_clause: $ => seq(
      'when:', $.text_line,
      'the system shall:', $.text_line
    ),

    acceptance: $ => prec.left(seq(
      'acceptance:',
      repeat1($.acc_clause)
    )),

    acc_clause: $ => seq(
      choice('given:', 'when:', 'then:'),
      $.text_line
    ),

    concept: $ => prec.left(seq(
      'Concept',
      $.identifier,
      ':',
      repeat1(choice($.field, $.prose))
    )),

    field: $ => seq(
      'field',
      $.identifier,
      optional(seq('(', $.type_expr, ')')),
      optional(seq(':', repeat1($.constraint)))
    ),

    type_expr: $ => choice(
      $.reference,
      seq('List', 'of', $.reference),
      seq('Optional', $.reference),
      seq('one', 'of:', $.variant_list)
    ),

    variant_list: $ => seq(
      $.identifier,
      repeat(seq(',', $.identifier))
    ),

    constraint: $ => choice(
      'unique',
      seq('default:', $.expression),
      seq('derived:', $.expression),
      seq('invariant:', $.expression),
      seq('at', 'least', $.number)
    ),

    behavior: $ => prec.left(seq(
      'Behavior', $.identifier, ':',
      repeat1(choice(
        $.implements_clause,
        $.behavior_body,
        $.prose
      ))
    )),

    implements_clause: $ => seq(
      'Implements', $.req_id, repeat(seq(',', $.req_id)), '.'
    ),

    behavior_body: $ => choice(
      seq('given:', $.text_line),
      seq('returns:', $.text_line),
      seq('requires:', $.text_line),
      seq('ensures:', $.text_line),
      $.ears_clause
    ),

    invariant: $ => prec.left(seq(
      'Invariant', $.identifier, ':',
      repeat1(choice(
        $.quantifier,
        $.prose
      ))
    )),

    quantifier: $ => seq(
      'for each', $.identifier, 'in', $.reference, ':'
    ),

    task: $ => prec.left(seq(
      '##',
      alias($.task_id, $.identifier),
      ':',
      token.immediate(/[^\n]+/),
      repeat1(choice(
        $.task_ref_list,
        $.task_field,
        $.prose
      ))
    )),

    task_id: $ => /TASK-[A-Z0-9-]+/,

    task_ref_list: $ => seq('[', $.req_id, repeat(seq(',', $.req_id)), ']'),

    task_field: $ => seq(
      choice('file:', 'tests:', 'depends:', 'status:', 'evidence:', 'context:'),
      $.text_line
    ),

    aesthetic: $ => prec.left(seq(
      'Aesthetic', $.identifier, ':',
      repeat1(choice(
        $.aesthetic_field,
        $.prose
      ))
    )),

    aesthetic_field: $ => seq(
      $.identifier, ':', optional('[~]'), $.text_line
    ),

    foreign_block: $ => seq(
      '```',
      alias(/[^\n]+/, $.language),
      optional(repeat(token.immediate(/[^\n]+/))),
      '```'
    ),

    text_line: $ => $.prose,

    expression: $ => $.prose,

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    reference: $ => seq('`', alias(/[^`]+/, $.identifier), '`'),

    string: $ => /"[^"]*"/,

    number: $ => /\d+/,

    comment: $ => token(seq('//', /[^\n]*/)),
  }
});