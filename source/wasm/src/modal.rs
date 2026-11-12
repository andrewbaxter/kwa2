use {
    crate::state::state,
    rooting::{
        El,
        WeakEl,
    },
};

pub fn remove_child(parent: &El, child: &El) {
    let children = parent.raw().children();
    for i in 0 .. children.length() {
        if children.item(i).unwrap().is_same_node(Some(&child.raw())) {
            parent.ref_splice(i as usize, 1, vec![]);
            return;
        }
    }
}

#[derive(Clone, Copy)]
enum ModalScope {
    Window,
    Page,
}

#[derive(Clone)]
pub struct ModalHandle {
    el: WeakEl,
    scope: ModalScope,
}

fn scope_root(scope: ModalScope) -> El {
    match scope {
        ModalScope::Window => return state().modal_root.get().unwrap().clone(),
        ModalScope::Page => return state().page_root.get().unwrap().clone(),
    }
}

impl ModalHandle {
    pub fn new(el: &El) -> ModalHandle {
        return ModalHandle {
            el: el.weak(),
            scope: ModalScope::Window,
        };
    }

    pub fn new_page(el: &El) -> ModalHandle {
        return ModalHandle {
            el: el.weak(),
            scope: ModalScope::Page,
        };
    }

    pub fn open(&self) {
        let Some(el) = self.el.upgrade() else {
            return;
        };
        scope_root(self.scope).ref_push(el);
    }

    pub fn close(&self) {
        let Some(el) = self.el.upgrade() else {
            return;
        };
        remove_child(&scope_root(self.scope), &el);
    }
}
