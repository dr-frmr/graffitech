use hex_color::HexColor;
use kinode_process_lib::NodeId;
use std::collections::HashMap;

pub struct World {
    pub drawings: HashMap<NodeId, Drawing>,
}

pub type Drawing = Vec<Point>;

pub struct Point {
    pub x: i64,
    pub y: i64,
    pub author: NodeId,
    color: HexColor,
}

mod autosurgeon_address {
    use autosurgeon::{Hydrate, HydrateError, Prop, ReadDoc, Reconciler};
    use kinode_process_lib::Address;
    pub(super) fn hydrate<'a, D: ReadDoc>(
        doc: &D,
        obj: &automerge::ObjId,
        prop: Prop<'a>,
    ) -> Result<Address, HydrateError> {
        let inner = String::hydrate(doc, obj, prop)?;
        inner.parse().map_err(|e| {
            HydrateError::unexpected(
                "a valid address",
                format!("an address which failed to parse due to {}", e),
            )
        })
    }

    pub(super) fn reconcile<R: Reconciler>(
        path: &Address,
        mut reconciler: R,
    ) -> Result<(), R::Error> {
        reconciler.str(path.to_string())
    }
}
