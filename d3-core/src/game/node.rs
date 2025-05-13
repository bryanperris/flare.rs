use core::{cmp::Ordering, ops::Range};
use std::{collections::BinaryHeap, rc::Weak};

use vector::Vector;

use super::{prelude::*, room::Room, RegionRef};

const MAX_NODE_SIZE: f32 = 20.0;

#[derive(Debug, Clone)]
pub struct NodeEdge {
    pub end_room: Option<WeakSharedMutRef<Room>>,
    pub end_index: usize,
    pub cost: i16,
    pub max_rad: f32
}

#[derive(Debug, Clone)]
pub struct Node {
    pub position: Vector,
    pub edges: Vec<NodeEdge>,
}

impl Node {
    pub fn quick_dist(&self, other_node: Node) -> f32 {
        let mut temp = (self.position.x - other_node.position.x).abs();
        temp += (self.position.y - other_node.position.y).abs();
        temp += (self.position.z - other_node.position.z).abs();

        temp
    }
}

pub struct NodePath {
    /// Collection of node indicies
    pub nodes: Vec<usize>
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderedNode {
    pub node: usize,
    pub parent_node: Option<usize>,
    pub cost: f32,
}

impl Eq for OrderedNode {

}

impl Ord for OrderedNode {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(core::cmp::Ordering::Equal)
    }
}

impl PartialOrd for OrderedNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl NodePath {
    pub fn find_path(&mut self, node_list_ref: &SharedMutRef<Vec<Node>>, start_room: &SharedMutRef<Room>, i: Option<usize>, j: Option<usize>, rad: f32) -> bool {
        let mut path: BinaryHeap<OrderedNode> = BinaryHeap::new();

        let i = i.unwrap();
        let j = j.unwrap();

        let start_node = OrderedNode {
            node: i,
            parent_node: None,
            cost: 0.0
        };

        let mut current_node: Option<OrderedNode> = None;

        let node_list = node_list_ref.borrow();

        assert!(i < node_list.len() && j < node_list.len());

        let mut ordered_list: Vec<Option<OrderedNode>> = std::iter::repeat_with(|| None)
        .take(node_list.len())
        .collect();

        path.push(start_node);

        current_node = path.pop();

        let mut found = false;

        while current_node.as_ref().is_some() {
            let cur_node = current_node.as_ref().unwrap();

            let node_index = cur_node.node;

            ordered_list[node_index] = current_node.clone();

            if node_index == j {
                self.update(&ordered_list, Some(i)..Some(j));
                found = true;
                break;
            }

            if !found {
                let num_edges = node_list[node_index].edges.len();

                for counter in 0..num_edges {
                    let end_room_ref = 
                        node_list[node_index].edges[counter].end_room
                        .as_ref().unwrap()
                        .upgrade().unwrap();

                    if !Rc::ptr_eq(&end_room_ref, start_room) {
                            continue;
                    }

                    let next_node = node_list[node_index].edges[counter].end_index;
                    let cost = node_list[node_index].edges[counter].cost;

                    assert!(cost > 0);

                    let new_cost = cur_node.cost + cost as f32;

                    let mut list_item_result = &mut ordered_list[node_index];

                    if list_item_result.is_some() && list_item_result.as_ref().unwrap().cost < new_cost {
                        continue;
                    }

                    if list_item_result.is_none() {
                        let list_item = OrderedNode {
                            node: next_node,
                            parent_node: Some(node_index),
                            cost: new_cost
                        };

                        ordered_list[next_node] = Some(list_item.clone());
                        path.push(list_item);
                    }
                    else {
                        let mut list_item = list_item_result.as_mut().unwrap();
                        list_item.cost = new_cost;
                        list_item.parent_node = Some(cur_node.node);
                    }
                }
            }

            current_node = path.pop();
        }

        found
    }

    fn update(&mut self, nodes: &Vec<Option<OrderedNode>>, range: Range<Option<usize>>) {
        let mut current_node = range.end;

        self.nodes.clear();

        while current_node.is_some() {
            let cur = current_node.unwrap();
            self.nodes.push(cur);
            current_node = nodes[cur].as_ref().unwrap().parent_node;
        }

        /* Reverse the list (so it is what we want) */
        let mut i = 0;
        while i < (self.nodes.len() >> 1) {
            let temp = self.nodes[i];
            let j = self.nodes.len() - i - 1;
            self.nodes[i] = self.nodes[j];
            self.nodes[j] = temp;

            i += 1;
        }
    }
}


#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VisState {
    No,
    Ok,
}

pub struct NodeVisibilityList {
    pub vis_list: Vec<VisState>
}

impl NodeVisibilityList {
    pub fn find_dir_local_local_visibile_node(&self, node_list_ref: &SharedMutRef<Vec<Node>>, position: &Vector, foward: &Vector, rad: f32) {
        let best_dot = -1.01;
        let closest_dist = 800.0;
        let closet_node: Option<usize> = None;
        let mut retry = false;

        let min_node_rad = if rad / 4.0 > MAX_NODE_SIZE {
            MAX_NODE_SIZE
        }
        else {
            rad / 4.0
        };

        let node_list = node_list_ref.borrow();

        'retry: loop {
            for i in 0..node_list.len() {
                let mut to = node_list[i].position - *position;
                let dist = Vector::normalize(&mut to);

                if dist < closest_dist {
                    let dot = foward.dot(to);

                    if dot > 0.0 || retry {
                        let mut node_size = 0.0;

                        if !retry || (retry && self.vis_list[i] == VisState::No) {
                            todo!()
                        }
                    }
                }

            }
            
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordered_nodes() {
        let mut heap = BinaryHeap::new();

        heap.push(OrderedNode {
            node: 1,    // usize
            parent_node: Some(0),  // usize
            cost: 2.5,  // f32
        });
        heap.push(OrderedNode {
            node: 2,
            parent_node: Some(1),
            cost: 1.5,
        });
        heap.push(OrderedNode {
            node: 3,
            parent_node: Some(2),
            cost: 3.0,
        });

        assert_eq!(
            heap.pop(),
            Some(OrderedNode {
                node: 2,
                parent_node: Some(1),
                cost: 1.5
            })
        );
        assert_eq!(
            heap.pop(),
            Some(OrderedNode {
                node: 1,
                parent_node: Some(0),
                cost: 2.5
            })
        );
        assert_eq!(
            heap.pop(),
            Some(OrderedNode {
                node: 3,
                parent_node: Some(2),
                cost: 3.0
            })
        );
        assert_eq!(heap.pop(), None);
    }
}
