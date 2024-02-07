import ApplicationServices
import Cocoa
import Foundation
import SwiftRs

// Credit: https://github.com/BenziAhamed/Menu-Bar-Search

class MenuBar {
    var menuBar: AXUIElement?
    let initState: AXError

    init(for app: NSRunningApplication) {
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var menuBarValue: CFTypeRef?
        initState = AXUIElementCopyAttributeValue(axApp, kAXMenuBarAttribute as CFString, &menuBarValue)
        if initState == .success {
            menuBar = (menuBarValue as! AXUIElement)
        }
    }

    func click(pathIndices: [Int]) {
        guard let menuBar = menuBar else {
            return
        }
        clickMenu(menu: menuBar, pathIndices: pathIndices, currentIndex: 0)
    }

    func load(_ options: MenuGetterOptions) -> [MenuItem] {
        guard let menuBar = menuBar else {
            return []
        }
        return MenuGetter.loadSync(menuBar: menuBar, options: options)
    }

    func loadAsync(_ options: MenuGetterOptions) -> [MenuItem] {
        guard let menuBar = menuBar else {
            return []
        }
        return MenuGetter.loadAsync(menuBar: menuBar, options: options)
    }
}

let virtualKeys = [
    0x24: "Enter", // kVK_Return
    0x4C: "KeypadEnter", // kVK_ANSI_KeypadEnter
    0x47: "KeypadClear", // kVK_ANSI_KeypadClear
    0x30: "Tab", // kVK_Tab
    0x31: "Space", // kVK_Space
    0x33: "Backspace", // kVK_Delete
    0x35: "Escape", // kVK_Escape
    0x39: "CapsLock", // kVK_CapsLock
    0x3F: "Fn", // kVK_Function
    0x7A: "F1", // kVK_F1
    0x78: "F2", // kVK_F2
    0x63: "F3", // kVK_F3
    0x76: "F4", // kVK_F4
    0x60: "F5", // kVK_F5
    0x61: "F6", // kVK_F6
    0x62: "F7", // kVK_F7
    0x64: "F8", // kVK_F8
    0x65: "F9", // kVK_F9
    0x6D: "F10", // kVK_F10
    0x67: "F11", // kVK_F11
    0x6F: "F12", // kVK_F12
    0x69: "F13", // kVK_F13
    0x6B: "F14", // kVK_F14
    0x71: "F15", // kVK_F15
    0x6A: "F16", // kVK_F16
    0x40: "F17", // kVK_F17
    0x4F: "F18", // kVK_F18
    0x50: "F19", // kVK_F19
    0x5A: "F20", // kVK_F20
    0x73: "Home", // kVK_Home
    0x74: "PageUp", // kVK_PageUp
    0x75: "Delete", // kVK_ForwardDelete
    0x77: "End", // kVK_End
    0x79: "PageDown", // kVK_PageDown
    0x7B: "ArrowLeft", // kVK_LeftArrow
    0x7C: "ArrowRight", // kVK_RightArrow
    0x7D: "ArrowDown", // kVK_DownArrow
    0x7E: "ArrowUp", // kVK_UpArrow
]

func decode(modifiers: Int) -> [String] {
    if modifiers == 0x18 { return ["Fn"] }
    var result = [String]()
    if (modifiers & 0x04) > 0 { result.append("Ctrl") }
    if (modifiers & 0x02) > 0 { result.append("Opt") }
    if (modifiers & 0x01) > 0 { result.append("Shift") }
    if (modifiers & 0x08) == 0 { result.append("Cmd") }
    return result
}

struct Shortcut: Codable {
    var mods: [String] = []
    var key: String
}

func getShortcut(_: String?, _ modifiers: Int, _ virtualKey: Int) -> Shortcut? {
    var shortcut: String?
    if virtualKey > 0 {
        if let lookup = virtualKeys[virtualKey] {
            shortcut = lookup
        }
    }
    let mods = decode(modifiers: modifiers)
    guard let shortcut else {
        return nil
    }
    return Shortcut(mods: mods, key: shortcut)
}

func getAttribute(element: AXUIElement, name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    AXUIElementCopyAttributeValue(element, name as CFString, &value)
    return value
}

func clickMenu(menu element: AXUIElement, pathIndices: [Int], currentIndex: Int) {
    guard let menuBarItems = getAttribute(element: element, name: kAXChildrenAttribute) as? [AXUIElement], menuBarItems.count > 0 else { return }
    let itemIndex = pathIndices[currentIndex]
    guard itemIndex >= menuBarItems.startIndex, itemIndex < menuBarItems.endIndex else { return }
    let child = menuBarItems[itemIndex]
    if currentIndex == pathIndices.count - 1 {
        AXUIElementPerformAction(child, kAXPressAction as CFString)
        return
    }
    guard let menuBar = getAttribute(element: child, name: kAXChildrenAttribute) as? [AXUIElement] else { return }
    clickMenu(menu: menuBar[0], pathIndices: pathIndices, currentIndex: currentIndex + 1)
}

func getMenuItems(
    forElement element: AXUIElement,
    menuItems: inout [MenuItem],
    path: [String] = [],
    pathIndices: [Int] = [],
    depth: Int = 0,
    options: MenuGetterOptions
) {
    // print(String(repeating: ".", count: depth), "🟢 getMenuItems for", path)
    guard depth < options.maxDepth else { return }
    guard let children = getAttribute(element: element, name: kAXChildrenAttribute) as? [AXUIElement], children.count > 0 else { return }
    var processedChildrenCount = 0
    for i in children.indices {
        let child = children[i]

        guard let enabled = getAttribute(element: child, name: kAXEnabledAttribute) as? Bool else { continue }

        // print(String(repeating: ".", count: depth + 1), "🔴 getMenuItems name:", getAttribute(element: child, name: kAXTitleAttribute))
        guard let title = getAttribute(element: child, name: kAXTitleAttribute) as? String else { continue }
        guard !title.isEmpty else { continue }
        let name = title.replacingOccurrences(of: "\n", with: " ").trimmingCharacters(in: CharacterSet.whitespaces)
        guard let children = getAttribute(element: child, name: kAXChildrenAttribute) as? [AXUIElement] else { continue }

        if options.dumpInfo {
            dumpInfo(element: child, name: name, depth: depth)
        }

        let menuPath = path + [name]
        if options.canIgnorePath(path: menuPath) { continue }

        if children.count == 1, enabled {
            // sub-menu item, scan children
            getMenuItems(
                forElement: children[0],
                menuItems: &menuItems,
                path: menuPath,
                pathIndices: pathIndices + [i],
                depth: depth + 1,
                options: options
            )
        } else {
            // if !options.appFilter.showDisabledMenuItems, !enabled { continue }

            if options.dumpInfo {
                print("➕ adding ", menuPath)
            }

            // not a sub menu, if we have a path to this item
            let cmd = getAttribute(element: child, name: kAXMenuItemCmdCharAttribute) as? String
            var modifiers = 0
            var virtualKey = 0
            if let m = getAttribute(element: child, name: kAXMenuItemCmdModifiersAttribute) {
                CFNumberGetValue((m as! CFNumber), CFNumberType.longType, &modifiers)
            }
            if let v = getAttribute(element: child, name: kAXMenuItemCmdVirtualKeyAttribute) {
                CFNumberGetValue((v as! CFNumber), CFNumberType.longType, &virtualKey)
            }

            var menuItem = MenuItem()
            menuItem.path = menuPath
            menuItem.pathIndices = pathIndices + [i]
            menuItem.shortcut = getShortcut(cmd, modifiers, virtualKey)
            menuItems.append(menuItem)

            processedChildrenCount += 1
            if processedChildrenCount > options.maxChildren {
                break
            }
        }
    }
}

func dumpInfo(element: AXUIElement, name: String, depth: Int) {
    let padding = " " + String(repeating: " |", count: depth - 1)
    print(padding, ":::", name, ":::")
    print(padding, "   ", element)
    func printAttributeInfo(_ header: String, _ attributes: [String]) {
        let values = attributes.compactMap { (name: String) -> (String, CFTypeRef)? in
            if let a = getAttribute(element: element, name: name) {
                return (name, a)
            }
            return nil
        }
        guard values.count > 0 else { return }
        print(padding, "    ", header)
        values.forEach { print(padding, "        ", $0.0, $0.1) }
    }

    printAttributeInfo("- informational attributes", [
        kAXRoleAttribute,
        kAXSubroleAttribute,
        kAXRoleDescriptionAttribute,
        kAXTitleAttribute,
        kAXDescriptionAttribute,
        kAXHelpAttribute,
    ])

    printAttributeInfo("- hierarchy or relationship attributes", [
        kAXParentAttribute,
        kAXChildrenAttribute,
        kAXSelectedChildrenAttribute,
        kAXVisibleChildrenAttribute,
        kAXWindowAttribute,
        kAXTopLevelUIElementAttribute,
        kAXTitleUIElementAttribute,
        kAXServesAsTitleForUIElementsAttribute,
        kAXLinkedUIElementsAttribute,
        kAXSharedFocusElementsAttribute,
    ])

    printAttributeInfo("- visual state attributes", [
        kAXEnabledAttribute,
        kAXFocusedAttribute,
        kAXPositionAttribute,
        kAXSizeAttribute,
    ])

    printAttributeInfo("- value attributes", [
        kAXValueAttribute,
        kAXValueDescriptionAttribute,
        kAXMinValueAttribute,
        kAXMaxValueAttribute,
        kAXValueIncrementAttribute,
        kAXValueWrapsAttribute,
        kAXAllowedValuesAttribute,
    ])

    printAttributeInfo("- text-specific attributes", [
        kAXSelectedTextAttribute,
        kAXSelectedTextRangeAttribute,
        kAXSelectedTextRangesAttribute,
        kAXVisibleCharacterRangeAttribute,
        kAXNumberOfCharactersAttribute,
        kAXSharedTextUIElementsAttribute,
        kAXSharedCharacterRangeAttribute,
    ])

    printAttributeInfo("- window, sheet, or drawer-specific attributes", [
        kAXMainAttribute,
        kAXMinimizedAttribute,
        kAXCloseButtonAttribute,
        kAXZoomButtonAttribute,
        kAXMinimizeButtonAttribute,
        kAXToolbarButtonAttribute,
        kAXProxyAttribute,
        kAXGrowAreaAttribute,
        kAXModalAttribute,
        kAXDefaultButtonAttribute,
        kAXCancelButtonAttribute,
    ])

    printAttributeInfo("- menu or menu item-specific attributes", [
        kAXMenuItemCmdCharAttribute,
        kAXMenuItemCmdVirtualKeyAttribute,
        kAXMenuItemCmdGlyphAttribute,
        kAXMenuItemCmdModifiersAttribute,
        kAXMenuItemMarkCharAttribute,
        kAXMenuItemPrimaryUIElementAttribute,
    ])

    printAttributeInfo("- application element-specific attributes", [
        kAXMenuBarAttribute,
        kAXWindowsAttribute,
        kAXFrontmostAttribute,
        kAXHiddenAttribute,
        kAXMainWindowAttribute,
        kAXFocusedWindowAttribute,
        kAXFocusedUIElementAttribute,
        kAXExtrasMenuBarAttribute,
    ])

    printAttributeInfo("- date/time-specific attributes", [
        kAXHourFieldAttribute,
        kAXMinuteFieldAttribute,
        kAXSecondFieldAttribute,
        kAXAMPMFieldAttribute,
        kAXDayFieldAttribute,
        kAXMonthFieldAttribute,
        kAXYearFieldAttribute,
    ])

    printAttributeInfo("- table, outline, or browser-specific attributes", [
        kAXRowsAttribute,
        kAXVisibleRowsAttribute,
        kAXSelectedRowsAttribute,
        kAXColumnsAttribute,
        kAXVisibleColumnsAttribute,
        kAXSelectedColumnsAttribute,
        kAXSortDirectionAttribute,
        kAXColumnHeaderUIElementsAttribute,
        kAXIndexAttribute,
        kAXDisclosingAttribute,
        kAXDisclosedRowsAttribute,
        kAXDisclosedByRowAttribute,
    ])

    printAttributeInfo("- matte-specific attributes", [
        kAXMatteHoleAttribute,
        kAXMatteContentUIElementAttribute,
    ])

    printAttributeInfo("- ruler-specific attributes", [
        kAXMarkerUIElementsAttribute,
        kAXUnitsAttribute,
        kAXUnitDescriptionAttribute,
        kAXMarkerTypeAttribute,
        kAXMarkerTypeDescriptionAttribute,
    ])

    printAttributeInfo("- miscellaneous or role-specific attributes", [
        kAXHorizontalScrollBarAttribute,
        kAXVerticalScrollBarAttribute,
        kAXOrientationAttribute,
        kAXHeaderAttribute,
        kAXEditedAttribute,
        kAXTabsAttribute,
        kAXOverflowButtonAttribute,
        kAXFilenameAttribute,
        kAXExpandedAttribute,
        kAXSelectedAttribute,
        kAXSplittersAttribute,
        kAXContentsAttribute,
        kAXNextContentsAttribute,
        kAXPreviousContentsAttribute,
        kAXDocumentAttribute,
        kAXIncrementorAttribute,
        kAXDecrementButtonAttribute,
        kAXIncrementButtonAttribute,
        kAXColumnTitleAttribute,
        kAXURLAttribute,
        kAXLabelUIElementsAttribute,
        kAXLabelValueAttribute,
        kAXShownMenuUIElementAttribute,
        kAXIsApplicationRunningAttribute,
        kAXFocusedApplicationAttribute,
        kAXElementBusyAttribute,
        kAXAlternateUIVisibleAttribute,
    ])
}

struct MenuGetterOptions {
    var maxDepth = 100
    var maxChildren = 100
    var specificMenuRoot: String?
    var dumpInfo = false
    var recache = false
    init() {}

    func canIgnorePath(path _: [String]) -> Bool {
        // print("not ignoring \(path)")
        return false
    }
}

enum MenuGetter {
    static func loadSync(menuBar: AXUIElement, options: MenuGetterOptions) -> [MenuItem] {
        var menuItems = [MenuItem]()
        guard let menuBarItems = getAttribute(element: menuBar, name: kAXChildrenAttribute) as? [AXUIElement],
              menuBarItems.count > 0 else { return [] }
        for i in menuBarItems.indices {
            let item = menuBarItems[i]
            guard let name = getAttribute(element: item, name: kAXTitleAttribute) as? String else { continue }

            if name == "Apple" { continue }
            if options.canIgnorePath(path: [name]) { continue }

            if let menuRoot = options.specificMenuRoot, name.lowercased() != menuRoot.lowercased() { continue }
            guard let children = getAttribute(element: item, name: kAXChildrenAttribute) as? [AXUIElement] else { continue }
            getMenuItems(
                forElement: children[0],
                menuItems: &menuItems,
                path: [name],
                pathIndices: [i],
                depth: 1,
                options: options
            )
        }
        return menuItems
    }

    static func loadAsync(menuBar: AXUIElement, options: MenuGetterOptions) -> [MenuItem] {
        var menuItems = [MenuItem]()
        let q: DispatchQueue
        if #available(macOS 10.10, *) {
            q = DispatchQueue(label: "folded-paper.menu-bar", qos: .userInteractive, attributes: .concurrent)
        } else {
            q = DispatchQueue(label: "folded-paper.menu-bar", attributes: .concurrent)
        }
        let group = DispatchGroup()
        guard let menuBarItems = getAttribute(element: menuBar, name: kAXChildrenAttribute) as? [AXUIElement],
              menuBarItems.count > 0 else { return [] }

        for i in menuBarItems.indices {
            let item = menuBarItems[i]
            guard let name = getAttribute(element: item, name: kAXTitleAttribute) as? String else { continue }

            if name == "Apple" { continue }
            if options.canIgnorePath(path: [name]) { continue }

            if let menuRoot = options.specificMenuRoot, name.lowercased() != menuRoot.lowercased() { continue }
            guard let children = getAttribute(element: item, name: kAXChildrenAttribute) as? [AXUIElement] else { continue }

            q.async(group: group) {
                var items = [MenuItem]()
                getMenuItems(
                    forElement: children[0],
                    menuItems: &items,
                    path: [name],
                    pathIndices: [i],
                    depth: 1,
                    options: options
                )
                q.async(group: group, flags: .barrier) {
                    menuItems.append(contentsOf: items)
                }
            }
        }
        _ = group.wait(timeout: .distantFuture)
        return menuItems
    }
}

struct MenuItem: Codable {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    var pathIndices: [Int] = []

    var shortcut: Shortcut? = nil

    var path: [String] = []

    init() {}
}

@_cdecl("menu_items")
public func getItems() -> SRData {
    let app = NSWorkspace.shared.menuBarOwningApplication
    guard let app = app else {
        print("no app :(")
        exit(0)
    }
    let menuBar = MenuBar(for: app)
    let menuItems = menuBar.load(MenuGetterOptions())
    let jsonEncoder = JSONEncoder()
    do {
        let jsonData = try [UInt8](jsonEncoder.encode(menuItems))
        return SRData(jsonData)
    } catch {
        return SRData([])
    }
}

@_cdecl("menu_item_select")
public func selectMenuItem(data: SRData) {
    let app = NSWorkspace.shared.menuBarOwningApplication
    guard let app = app else {
        print("no app :(")
        exit(0)
    }
    let jsonDecoder = JSONDecoder()
    do {
        let indices = try jsonDecoder.decode([Int].self, from: Data(data.toArray()))
        let menuBar = MenuBar(for: app)
        menuBar.click(pathIndices: indices)
    } catch {
        return
    }
}
