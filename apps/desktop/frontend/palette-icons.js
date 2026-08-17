// Professional, CATIA-like technical palette symbols.
// This is presentation-only: semantic eligibility and creation remain Rust-owned.
paletteSymbol = function paletteSymbolTechnical(item) {
  const symbols = {
    Block: '▱',
    AssociationBlock: '⧉',
    InterfaceBlock: '⧄',
    ConstraintBlock: '⧈',
    ValueType: '◈',
    DataType: '▤',
    PrimitiveType: '◻',
    Enumeration: '≣',
    Signal: '↯',
    Unit: '⊥',
    QuantityKind: '⊙',
    InstanceSpecification: '⧉',
    Comment: '▰',

    PartProperty: '▣',
    ReferenceProperty: '↗',
    ValueProperty: '◇',
    FlowProperty: '⇄',
    ConstraintProperty: '⊞',
    ProxyPort: '⊡',
    FullPort: '▣',
    Operation: '⌁',
    Reception: '⇥',
    Parameter: '↦',
    EnumerationLiteral: '•',
    Slot: '◫',

    Association: '──',
    Aggregation: '◇─',
    Composition: '◆─',
    Generalization: '─▷',
    Dependency: '┄➤',
    Realization: '┄▷',
  };
  return symbols[item.semantic_kind] || symbols[item.relationship_kind] || '·';
};
