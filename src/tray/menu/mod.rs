pub mod about;
pub mod handler;
pub mod item;

use std::collections::HashMap;
use std::rc::Rc;

use log::error;
use tray_icon::menu::{CheckMenuItem, MenuId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MenuGroup {
    // GroupMulti
    // ... add more groups here as needed
    // GroupSingle
    WindowPosition,
    MonitorSelector,
    IndicatorIcon,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MenuKind {
    /// 普通菜单项（只处理点击，不需要状态）
    Normal,

    /// 单个 CheckMenu（不分组）
    CheckSingle,

    /// 分组单选：组内只允许 1 个被选中，若存在默认菜单则全部取消时自动回到勾选默认菜单
    GroupSingle(MenuGroup, /* default menu */ Option<MenuId>),

    /// 分组多选：组内允许任意勾选（互不影响）
    GroupMulti(MenuGroup),
}

#[derive(Clone)]
pub struct MenuManager {
    id_to_menu: HashMap<Rc<MenuId>, Rc<CheckMenuItem>>,
    id_to_kind: HashMap<Rc<MenuId>, Rc<MenuKind>>,
    kind_to_menus: HashMap<Rc<MenuKind>, HashMap<Rc<MenuId>, Rc<CheckMenuItem>>>,
}

impl MenuManager {
    pub fn new() -> Self {
        Self {
            id_to_menu: HashMap::new(),
            id_to_kind: HashMap::new(),
            kind_to_menus: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: MenuId, kind: MenuKind, check_menu: Option<CheckMenuItem>) {
        let id = Rc::new(id);
        let kind = Rc::new(kind);
        self.id_to_kind.insert(id.clone(), kind.clone());

        if let Some(menu) = check_menu {
            let menu = Rc::new(menu);
            self.id_to_menu.insert(id.clone(), menu.clone());
            self.kind_to_menus.entry(kind).or_default().insert(id, menu);
        }
    }

    // pub fn remove(&mut self, id: &MenuId) {
    //     self.id_to_menu.remove(id);
    //     self.id_to_kind
    //         .remove(id)
    //         .and_then(|k| self.kind_to_menus.remove(&k));
    // }

    pub fn get_menu_by_id(&self, id: &MenuId) -> Option<&CheckMenuItem> {
        self.id_to_menu.get(id).map(|rc| rc.as_ref())
    }

    fn get_kind_by_id(&self, id: &MenuId) -> Option<&MenuKind> {
        self.id_to_kind.get(id).map(|v| &**v)
    }

    pub fn get_menus_by_kind(
        &self,
        kind: &MenuKind,
    ) -> Option<&HashMap<Rc<MenuId>, Rc<CheckMenuItem>>> {
        self.kind_to_menus.get(kind)
    }

    fn get_menus_by_id(&self, id: &MenuId) -> Option<&HashMap<Rc<MenuId>, Rc<CheckMenuItem>>> {
        self.get_kind_by_id(id)
            .and_then(|k| self.get_menus_by_kind(k))
    }

    /// 为指定菜单项设置点击回调。
    ///
    /// 根据菜单项的类型，回调函数会收到不同的参数：
    ///
    /// - **普通菜单项（Normal）**：  
    ///   回调为 `callback(true, None)` —— 表示这是一个不可勾选的 `MenuItem`
    ///
    /// - **独立勾选项（CheckSingle）**：  
    ///   回调为 `callback(false, Some((Some(menu), None)))`
    ///
    /// - **多选分组（GroupMulti）**：  
    ///   回调为 `callback(false, Some((Some(menu), Some(group))))`
    ///
    /// - **单选分组（GroupSingle）**：  
    ///     - 存在默认菜单：点击后自动取消同组其他项；若试图取消最后一项，则自动选中默认菜单项
    ///       回调为 `callback(false, Some((Some(menu), Some(..))))`
    ///     - 不存在默认菜单：点击后自动取消同组其他项，若试图取消最后一项，则无特殊处理
    ///       回调为 `callback(false, Some(None, Some(..))))` —— 该分组的 `CheckMenu` 全部为 `Not check`
    /// # 参数
    /// - `id`: 要绑定回调的菜单项 ID  
    /// - `callback`: 回调函数，签名：`Fn(bool, Option<(Option<CheckMenuItem>, Option<MenuGroup>)>)`
    ///     - `bool`: 点击的菜单是否为`MenuItem`
    ///     - `Option<(Option<CheckMenuItem>, Option<MenuGroup>)>`:
    ///         - `None`: 点击的菜单非`CheckMenu`
    ///         - `Some(..)`: 点击的菜单是`CheckMenu`
    ///             - `Option<CheckMenuItem>`
    ///                 - `None`: 被点击的`CheckMenu`存在分组，但无默认菜单，返回`None`表示该组的全部`CheckMenu`是`Not check`
    ///                 - `Some(..)`:
    ///                   1. 被点击的`CheckMenu`的分组中存在默认菜单，返回状态是`Checked`的`CheckMenu`
    ///                   2. 返回无分组且被点击的`CheckMenu`
    ///             - `Option<MenuGroup>`
    ///                 - `None`: 被点击的`CheckMenu`无分组
    ///                 - `Some(..)`: 被点击的`CheckMenu`存在分组，返回分组类型
    pub fn handler<F>(&mut self, id: &MenuId, callback: F)
    where
        F: Fn(bool, Option<(Option<CheckMenuItem>, Option<MenuGroup>)>),
    {
        let Some(kind) = self.get_kind_by_id(id) else {
            error!("Failed to get '{}' menu kind", id.0);
            return;
        };

        match kind.clone() {
            MenuKind::Normal => callback(true, None),
            MenuKind::CheckSingle => callback(
                false,
                self.get_menu_by_id(id).map(|m| (Some(m.clone()), None)),
            ),
            MenuKind::GroupMulti(group) => callback(
                false,
                self.get_menu_by_id(id)
                    .map(|m| (Some(m.clone()), Some(group))),
            ),
            MenuKind::GroupSingle(group, default) => {
                // 组内单选 + 全取消回默认（并触发默认项的回调）
                self.handler_group_single_select(id, default, group, callback);
            }
        }
    }

    fn handler_group_single_select<F>(
        &mut self,
        id: &MenuId,
        default_id: Option<MenuId>,
        group: MenuGroup,
        callback: F,
    ) where
        F: Fn(bool, Option<(Option<CheckMenuItem>, Option<MenuGroup>)>),
    {
        let Some(click_menu) = self.get_menu_by_id(id) else {
            error!("No kind of menu found: {}", id.0);
            return;
        };

        let Some(menus) = self.get_menus_by_id(id) else {
            error!(
                "Failed to find the menu({}) group from the kind: {:?}",
                id.0,
                self.get_kind_by_id(id)
            );
            return;
        };

        let click_menu_state = click_menu.is_checked();

        let (is_checked_menu_id, is_checked_menu) = if click_menu_state {
            (id, click_menu)
        } else {
            let Some(default_id) = default_id else {
                return callback(false, Some((None, Some(group))));
            };

            let Some(default_menu) = menus.get(&default_id) else {
                error!("Failed to find the default menu menu for that '{group:?}'");
                return;
            };

            default_menu.set_checked(true);
            (&default_id.clone(), default_menu.as_ref())
        };

        menus
            .iter()
            .filter(|(menu_id, _)| menu_id.as_ref().ne(is_checked_menu_id))
            .for_each(|(_, check_menu)| check_menu.set_checked(false));

        callback(false, Some((Some(is_checked_menu.clone()), Some(group))));
    }
}
