module.exports = grammar({
  name: 'topos',

  extras: $ => [
    /\s+/,
    $.comment,
  ],

  conflicts: $ => [
    [$.section, $.prose],
    [$._section_content, $.prose],
  ],

  rules: {
    source_file: $ => repeat($._item),

    _item: $ => choice(
      $.spec_def,
      $.import_def,
      $.section,
      $.prose
    ),

    spec_def: $ => seq(token(prec(10, 'spec')), $.identifier),

    import_def: $ => seq(
      token(prec(10, 'import')),
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

    requirement: $ => seq(
      '##',
      alias($.req_id, $.identifier),
      ':',
      token.immediate(/[^\n]+/),
      repeat($.ears_clause),
      optional($.acceptance)
    ),

    req_id: $ => /REQ-[A-Z0-9-]+/,

    ears_clause: $ => seq(
      token(prec(10, 'when:')),
      $.text_line,
      token(prec(10, 'the system shall:')),
      $.text_line
    ),

    acceptance: $ => seq(
      token(prec(10, 'acceptance:')),
      repeat1($.acc_clause)
    ),

    acc_clause: $ => seq(
      choice(token(prec(10, 'given:')), token(prec(10, 'when:')), token(prec(10, 'then:'))),
      $.text_line
    ),

    concept: $ => seq(
      token(prec(10, 'Concept')),
      $.identifier,
      ':',
      repeat1($.field)
    ),

    field: $ => seq(
      token(prec(10, 'field')),
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
      seq('default:', $.text_line),
      seq('derived:', $.text_line),
      seq('invariant:', $.text_line),
      seq('at least', $.number)
    ),

    behavior: $ => seq(
      token(prec(10, 'Behavior')),
      $.identifier,
      ':',
      optional($.implements_clause),
      repeat1($.behavior_rule)
    ),

    implements_clause: $ => seq(
      'Implements',
      $.req_id,
      repeat(seq(',', $.req_id)),
      '.'
    ),

    behavior_rule: $ => choice(
      seq('given:', $.text_line),
      seq('returns:', $.text_line),
      seq('requires:', $.text_line),
      seq('ensures:', $.text_line),
      $.ears_clause
    ),

    invariant: $ => seq(
      token(prec(10, 'Invariant')),
      $.identifier,
      ':',
      optional($.quantifier),
      $.text_line
    ),

    quantifier: $ => seq(
      'for each',
      $.identifier,
      'in',
      $.reference,
      ':'
    ),

    task: $ => seq(
      '##',
      alias($.task_id, $.identifier),
      ':',
      token.immediate(/[^\n]+/),
      optional(seq('[', $.req_id, repeat(seq(',', $.req_id)), ']')), 
      repeat1($.task_field)
    ),

    task_id: $ => /TASK-[A-Z0-9-]+/,

    task_field: $ => seq(
      choice('file:', 'tests:', 'depends:', 'status:', 'evidence:', 'context:'),
      $.text_line
    ),

    aesthetic: $ => seq(
      token(prec(10, 'Aesthetic')),
      $.identifier,
      ':',
      repeat1($.aesthetic_field)
    ),

    aesthetic_field: $ => seq(
      $.identifier,
      ':',
      optional('[~]') ,
      $.text_line
    ),

    foreign_block: $ => seq(
      '```',
      alias(/[a-z]+/, $.language),
      optional(repeat(token.immediate(/[^\n]/))),
      '```'
    ),

    prose: $ => prec(-1, /[^\s#\n][^\n#]*/),

    text_line: $ => /[^\n]+/, 

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    reference: $ => seq('`', alias(/[^`]+/, $.identifier), '`'),

    string: $ => /"[^"]*"/,

    number: $ => /\d+/,

    comment: $ => token(seq('//', /[^\n]*/)),
  }
});
