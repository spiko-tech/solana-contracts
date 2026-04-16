import {
    type Codama,
    bottomUpTransformerVisitor,
    structFieldTypeNode,
    numberTypeNode,
    assertIsNode,
    isNode,
} from 'codama';

/**
 * Adds a `version: u8` field to account structs (after discriminator).
 *
 * All Spiko accounts follow the layout: [discriminator: u8][version: u8][data...].
 * The Codama IDL only captures what's in the Rust struct (which starts after disc+version),
 * plus the synthetic discriminator field. This transform inserts the version field.
 */
export function appendAccountVersion(codama: Codama): Codama {
    codama.update(
        bottomUpTransformerVisitor([
            {
                select: '[accountNode]',
                transform: (node) => {
                    assertIsNode(node, 'accountNode');

                    if (isNode(node.data, 'structTypeNode')) {
                        const fields = node.data.fields;
                        const discriminatorIndex = fields.findIndex(
                            (f) => f.name === 'discriminator',
                        );

                        const versionField = structFieldTypeNode({
                            name: 'version',
                            type: numberTypeNode('u8'),
                        });

                        const updatedFields =
                            discriminatorIndex >= 0
                                ? [
                                      ...fields.slice(0, discriminatorIndex + 1),
                                      versionField,
                                      ...fields.slice(discriminatorIndex + 1),
                                  ]
                                : [versionField, ...fields];

                        const updatedNode = {
                            ...node,
                            data: {
                                ...node.data,
                                fields: updatedFields,
                            },
                        };

                        if (node.size !== undefined) {
                            return {
                                ...updatedNode,
                                size: (node.size ?? 0) + 1,
                            };
                        }

                        return updatedNode;
                    }

                    return node;
                },
            },
        ]),
    );
    return codama;
}
