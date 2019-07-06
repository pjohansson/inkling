use crate::line::{Line, Choice, ParsedLine};

#[derive(Debug)]
struct Node {
    items: Vec<NodeItem>,
    choice: Option<Choice>,
}

impl Node {
    fn from_lines(lines: &[ParsedLine]) -> Node {
        let mut index = 0;
        let mut level = 0;

        let mut root = Node {
            choice: None,
            items: Vec::new(),
        };

        // let items = parse_at_level(&mut index, &mut level, lines);
        let items = parse_at_level(&mut root, &mut index, &mut level, lines);

        // dbg!(&root);

        root
    }
}

/// Parse lines until there is a change in current level. If the new level is higher,
/// call this function recursively for it and add the items as a root node. If the new
/// level is lower, return the items at once.
fn parse_at_level(
    node: &mut Node,
    index: &mut usize,
    current_level: &mut u8,
    lines: &[ParsedLine]
){
    let start_index = *index;
    let mut items = Vec::new();

    while *index < lines.len() {
        let line = &lines[*index];

        *index += 1;

        match line {

            ParsedLine::Line(line) => {
                items.push(NodeItem::Line(line.clone()));
            },

            ParsedLine::Choice { ref level, ref choice } => {
                if level > current_level {
                    *current_level = *level;
                    *index -= 1;
                    node.items.extend(items.drain(..));

                    let mut subnode = Node { choice: None, items: Vec::new() };

                    parse_at_level(&mut subnode, index, current_level, lines);

                    node.items.push(NodeItem::Node(Box::new(subnode)));
                }

                else if level == current_level && *index > start_index {
                    let subnode = Node { choice: Some(choice.clone()), items };

                    node.items.push(NodeItem::Node(Box::new(subnode)));

                    items = Vec::new();
                }

                else if level < current_level {
                    *current_level = *level;
                    break;
                }
            },


            // ParsedLine::Choice { level, choice } => {
            //     if *level > *current_level {
            //         // Nest into the upper layer to create the sub node
            //         *current_level = *level;
            //         *index -= 1;
            //
            //         let subnode_items = parse_at_level(index, current_level, lines);
            //
            //         eprintln!("returned to level {}, got items {:?}", *current_level, &subnode_items);
            //
            //         let subnode = Node { choice: None, items: subnode_items };
            //
            //         items.push(NodeItem::Node(Box::new(subnode)));
            //
            //     } else if *level == *current_level {
            //         eprintln!("found choice at same level! returning current items");
            //         items.push(NodeItem)
            //         *index -= 1;
            //         break;
            //
            //     } else {
            //         // Finalize the current node
            //         break;
            //     }



                // if *level > *current_level {
                //     *current_level = *level;
                //     let node_items = parse_at_level(index, current_level, lines);
                //     let node = Node { choice: Some(choice.clone()), items: node_items };
                //     items.push(NodeItem::Node(Box::new(node)));
                // } else if *level == *current_level {
                //
                // } else {
                //     *index -= 1;
                //     break;
                // }
            _ => (),
        }
    }

    node.items.extend(items.drain(..));
}

#[derive(Debug)]
enum NodeItem {
    Line(Line),
    Node(Box<Node>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fmt,
        ops::Index,
        str::FromStr,
    };

    impl  Node {
        fn display(&self) -> String {
            let mut buffer = String::new();

            self.display_indent(&mut buffer, 0);

            buffer
        }

        fn display_indent(&self, buffer: &mut String, level: usize) {
            let indent = format!("{:width$}", ' ', width=4 * level);
            for line in &self.items {
                buffer.push_str(&indent);
                match line {
                    NodeItem::Line(line) => {
                        buffer.push_str("Line\n");
                    },
                    NodeItem::Node(node) => {
                        if node.choice.is_none() {
                            buffer.push_str("MultiChoice [\n");
                        } else {
                            buffer.push_str("Choice [\n");
                        }
                        node.display_indent(buffer, level + 1);
                        buffer.push_str(&indent);
                        buffer.push_str("]\n");
                    }
                    _ => (),
                }
            }
        }
    }

    impl NodeItem {
        fn len(&self) -> usize {
            match self {
                NodeItem::Node(item) => item.items.len(),
                _ => panic!("expected a `Node` but found {:?}", self),
            }
        }
    }

    impl Index<usize> for NodeItem {
        type Output = Self;

        fn index(&self, index: usize) -> &Self::Output {
            match self {
                NodeItem::Node(item) => &item.items[index],
                _ => panic!("expected a `Node` but found {:?}", self),
            }
        }
    }


    #[test]
    fn flat_structure_parses_into_flat_node() {
        let empty_line = Line::from_str("").unwrap();
        let lines = vec![
            ParsedLine::Line(empty_line.clone()),
            ParsedLine::Line(empty_line.clone()),
        ];

        let root = Node::from_lines(&lines);

        assert_eq!(root.items.len(), 2);
    }

    #[test]
    fn choices_are_grouped_into_nodes() {
        let empty_line = Line::from_str("").unwrap();
        let empty_choice = Choice { selection_text: String::new(), line: empty_line.clone() };

        // Corresponding structure:
        // 0 (choice root)
        //   0.0 (choice)
        //   0.1 (choice)

        let lines = vec![
            ParsedLine::Choice { level: 1, choice: empty_choice.clone() },
            ParsedLine::Choice { level: 1, choice: empty_choice.clone() },
        ];

        let root = Node::from_lines(&lines);

        assert_eq!(root.items.len(), 1, "more than a single node was created");
        assert_eq!(root.items[0].len(), 2, "the level 1 node does not have exactly 2 children (choices)");
        assert_eq!(root.items[0][0].len(), 0, "choice 1 does not have 0 zero children");
        assert_eq!(root.items[0][1].len(), 0, "choice 2 does not have 0 zero children");
    }

    #[test]
    fn lines_below_choices_belong_to_them() {
        let empty_line = Line::from_str("").unwrap();
        let empty_choice = Choice { selection_text: String::new(), line: empty_line.clone() };

        // Corresponding structure:
        // 0 (choice root)
        //   0.0 (choice)
        //   0.0.1 (line)

        let lines = vec![
            ParsedLine::Choice { level: 1, choice: empty_choice.clone() },
            ParsedLine::Line(empty_line.clone()),
        ];

        let root = Node::from_lines(&lines);

        eprintln!("{}", root.display());

        assert_eq!(root.items.len(), 1, "more than a single node was created");
        assert_eq!(root.items[0].len(), 1, "the level 1 node does not have exactly 1 children (choices)");
        assert_eq!(root.items[0][0].len(), 1, "choice 1 does not have 1 children");
    }

    // #[test]
    fn flat_line_vector_parses_into_nested_structure() {
        let empty_line = Line::from_str("").unwrap();
        let empty_choice = Choice { selection_text: String::new(), line: empty_line.clone() };

        // Corresponding structure:
        //
        // 0 (line)
        // 1 (line)
        // 2 (choice)
        //    2.0 (line)
        //    2.1 (line)
        // 3 (choice)
        //   3.0 (line)
        //   3.1 (choice)
        //      3.1.0 (line)
        //      3.1.1 (line)
        //   3.2 (choice)
        //      3.2.0 (line)
        // 4 (choice)
        //   4.0 (line)

        let lines = vec![
            ParsedLine::Line(empty_line.clone()),
            ParsedLine::Line(empty_line.clone()),

            ParsedLine::Choice { level: 1, choice: empty_choice.clone() },
            ParsedLine::Line(empty_line.clone()),
            ParsedLine::Line(empty_line.clone()),

            ParsedLine::Choice { level: 1, choice: empty_choice.clone() },
            ParsedLine::Line(empty_line.clone()),

            ParsedLine::Choice { level: 2, choice: empty_choice.clone() },
            ParsedLine::Line(empty_line.clone()),
            ParsedLine::Line(empty_line.clone()),

            ParsedLine::Choice { level: 2, choice: empty_choice.clone() },
            ParsedLine::Line(empty_line.clone()),

            ParsedLine::Choice { level: 3, choice: empty_choice.clone() },
            ParsedLine::Line(empty_line.clone()),

            ParsedLine::Choice { level: 1, choice: empty_choice.clone() },
            ParsedLine::Line(empty_line.clone()),
        ];

        let root = Node::from_lines(&lines);

        eprintln!("{}", root.display());

        assert_eq!(root.items.len(), 5);

        assert_eq!(root.items[2].len(), 2);
        assert_eq!(root.items[3].len(), 3);
        assert_eq!(root.items[3][1].len(), 2);
        assert_eq!(root.items[3][2].len(), 2);
        assert_eq!(root.items[3][2][1].len(), 1);
        assert_eq!(root.items[4].len(), 1);
    }
    //
    // #[test]
    // fn gather_markers_shortcuts_branching() {
    //     let empty_line = Line::from_str("").unwrap();
    //     let empty_choice = Choice { selection_text: String::new(), line: empty_line.clone() };
    //
    //     // Corresponding structure:
    //     //
    //     // 0 (line)
    //     // 1 (line)
    //     // 2 (choice)
    //     //    2.0 (line)
    //     //    2.1 (line)
    //     // 3 (choice)
    //     //   3.0 (line)
    //     //   3.1 (choice)
    //     //      3.1.0 (line)
    //     //      3.1.1 (line)
    //     //   3.2 (choice)
    //     //      3.2.0 (line)
    //     // 4 (choice)
    //     //   4.0 (line)
    //
    //     let lines = vec![
    //         ParsedLine::Line(empty_line.clone()),
    //         ParsedLine::Line(empty_line.clone()),
    //
    //         ParsedLine::Choice { level: 1, choice: empty_choice.clone() },
    //         ParsedLine::Line(empty_line.clone()),
    //         ParsedLine::Line(empty_line.clone()),
    //
    //         ParsedLine::Choice { level: 1, choice: empty_choice.clone() },
    //         ParsedLine::Line(empty_line.clone()),
    //
    //         ParsedLine::Choice { level: 2, choice: empty_choice.clone() },
    //         ParsedLine::Line(empty_line.clone()),
    //         ParsedLine::Line(empty_line.clone()),
    //
    //         ParsedLine::Choice { level: 2, choice: empty_choice.clone() },
    //         ParsedLine::Line(empty_line.clone()),
    //
    //         ParsedLine::Choice { level: 3, choice: empty_choice.clone() },
    //         ParsedLine::Line(empty_line.clone()),
    //
    //         ParsedLine::Choice { level: 1, choice: empty_choice.clone() },
    //         ParsedLine::Line(empty_line.clone()),
    //     ];
    //
    //     let root = Node::from_lines(&lines);
    //     assert_eq!(root.items.len(), 5);
    //
    //     assert_eq!(root.items[2].len(), 2);
    //     assert_eq!(root.items[3].len(), 3);
    //     assert_eq!(root.items[3][1].len(), 2);
    //     assert_eq!(root.items[3][2].len(), 2);
    //     assert_eq!(root.items[3][2][1].len(), 1);
    //     assert_eq!(root.items[4].len(), 1);
    // }
}
