/*
 This source file is part of the Loungy open source project

 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License

 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
*/

// Original implementation: https://github.com/BenziAhamed/Menu-Bar-Search

import ApplicationServices
import Cocoa
import Foundation

final class MenuBar {
    enum Error: Swift.Error, CustomStringConvertible {
        case noAppFound
        case menuNotLoaded
        case failToConvert

        var description: String {
            switch self {
            case .noAppFound:
                return "No App Found"
            case .menuNotLoaded:
                return "App Menu couldn't be loaded"
            case .failToConvert:
                return "Failed to convert menu data"
            }
        }
    }

    private let virtualKeys = [
        0x24: "enter", // kVK_Return
        0x4C: "enter", // kVK_ANSI_KeypadEnter
        0x47: "enter", // kVK_ANSI_KeypadClear
        0x30: "tab", // kVK_Tab
        0x31: "space", // kVK_Space
        0x33: "backspace", // kVK_Delete
        0x35: "escape", // kVK_Escape
        0x39: "caps", // kVK_CapsLock
        0x3F: "function", // kVK_Function
        0x7A: "f1", // kVK_F1
        0x78: "f2", // kVK_F2
        0x63: "f3", // kVK_F3
        0x76: "f4", // kVK_F4
        0x60: "f5", // kVK_F5
        0x61: "f6", // kVK_F6
        0x62: "f7", // kVK_F7
        0x64: "f8", // kVK_F8
        0x65: "f9", // kVK_F9
        0x6D: "f10", // kVK_F10
        0x67: "f11", // kVK_F11
        0x6F: "f12", // kVK_F12
        0x69: "f13", // kVK_F13
        0x6B: "f14", // kVK_F14
        0x71: "f15", // kVK_F15
        0x6A: "f16", // kVK_F16
        0x40: "f17", // kVK_F17
        0x4F: "f18", // kVK_F18
        0x50: "f19", // kVK_F19
        0x5A: "f20", // kVK_F20
        0x73: "home", // kVK_Home
        0x74: "pageup", // kVK_PageUp
        0x75: "delete", // kVK_ForwardDelete
        0x77: "end", // kVK_End
        0x79: "pageup", // kVK_PageDown
        0x7B: "left", // kVK_LeftArrow
        0x7C: "right", // kVK_RightArrow
        0x7D: "down", // kVK_DownArrow
        0x7E: "up", // kVK_UpArrow
    ]

    private var menuBar: AXUIElement?
    private let state: AXError

    init() throws {
        let app = NSWorkspace.shared.menuBarOwningApplication

        guard let app else {
            throw Error.noAppFound
        }

        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var menuBarValue: CFTypeRef?

        state = AXUIElementCopyAttributeValue(
            axApp,
            kAXMenuBarAttribute as CFString,
            &menuBarValue
        )

        if state == .success {
            menuBar = (menuBarValue as! AXUIElement)
        }
    }

    func listItems(options: Options = .init()) throws -> [UInt8] {
        let encoder = JSONEncoder()

        do {
            guard let menuBar else {
                throw Error.menuNotLoaded
            }

            let items = load(
                from: menuBar,
                using: options
            )

            let data = try [UInt8](encoder.encode(items))

            return data
        }
        catch {
            throw Error.failToConvert
        }
    }

    func selectMenu(from data: Data) throws {
        let decoder = JSONDecoder()

        do {
            let indices = try decoder.decode([Int].self, from: data)

            guard let menuBar else {
                throw Error.menuNotLoaded
            }

            click(
                menu: menuBar,
                pathIndices: indices,
                currentIndex: 0
            )
        }
        catch {
            throw Error.failToConvert
        }
    }
}

// MARK: - Private Methods

private extension MenuBar {

    func decode(modifiers: Int) -> Modifiers {
        if modifiers == 0x18 {
            return Modifiers(
                function: true
            )
        }

        return Modifiers(
            control: (modifiers & 0x04) > 0,
            alt: (modifiers & 0x02) > 0,
            shift: (modifiers & 0x01) > 0,
            platform: (modifiers & 0x08) == 0
        )
    }

    func shortcut(_: String?, _ modifiers: Int, _ virtualKey: Int) -> Shortcut? {
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

        return Shortcut(
            modifiers: mods,
            key: shortcut
        )
    }

    func click(menu element: AXUIElement, pathIndices: [Int], currentIndex: Int) {
        guard
            let menuBarItems = element.attribute(for: kAXChildrenAttribute) as? [AXUIElement],
            menuBarItems.count > 0
        else {
            return
        }

        let itemIndex = pathIndices[currentIndex]

        guard
            itemIndex >= menuBarItems.startIndex,
            itemIndex < menuBarItems.endIndex
        else {
            return
        }

        let child = menuBarItems[itemIndex]

        if currentIndex == pathIndices.count - 1 {
            AXUIElementPerformAction(child, kAXPressAction as CFString)
            return
        }

        guard let menuBar = child.attribute(for: kAXChildrenAttribute) as? [AXUIElement] else {
            return
        }

        click(
            menu: menuBar[0],
            pathIndices: pathIndices,
            currentIndex: currentIndex + 1
        )
    }

    func menuItems(
        for element: AXUIElement,
        menuItems: inout [MenuItem],
        path: [String] = [],
        pathIndices: [Int] = [],
        depth: Int = 0,
        options: Options
    ) {
        let children = element.attribute(for: kAXChildrenAttribute) as? [AXUIElement]

        guard
            depth < options.maxDepth,
            let children,
            children.count > 0
        else {
            return
        }

        var processedChildrenCount = 0

        for (index, child) in children.enumerated() {
            guard
                let enabled = child.attribute(for: kAXEnabledAttribute) as? Bool,
                var title = child.attribute(for: kAXTitleAttribute) as? String,
                title.isNotEmpty
            else {
                continue
            }

            title.clearedNewLines()

            guard let children = child.attribute(for: kAXChildrenAttribute) as? [AXUIElement] else {
                continue
            }

            if options.dumpInfo {
                dumpInfo(element: child, name: title, depth: depth)
            }

            let menuPath = path + [title]

            if options.canIgnorePath(path: menuPath) {
                continue
            }

            if children.count == 1, enabled {
                // sub-menu item, scan children
                self.menuItems(
                    for: children[0],
                    menuItems: &menuItems,
                    path: menuPath,
                    pathIndices: pathIndices + [index],
                    depth: depth + 1,
                    options: options
                )
            }
            else {
                if options.dumpInfo {
                    print("➕ adding ", menuPath)
                }

                // not a sub menu, if we have a path to this item
                let command = child.attribute(for: kAXMenuItemCmdCharAttribute) as? String
                var modifiers = 0
                var virtualKey = 0

                if let modifier = child.attribute(for: kAXMenuItemCmdModifiersAttribute) {
                    CFNumberGetValue((modifier as! CFNumber), CFNumberType.longType, &modifiers)
                }

                if let key = child.attribute(for: kAXMenuItemCmdVirtualKeyAttribute) {
                    CFNumberGetValue((key as! CFNumber), CFNumberType.longType, &virtualKey)
                }

                var menuItem = MenuItem()
                menuItem.path = menuPath
                menuItem.pathIndices = pathIndices + [index]
                menuItem.shortcut = shortcut(command, modifiers, virtualKey)

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
            let values = attributes.compactMap { (name: String) -> (name: String, reference: CFTypeRef)? in
                guard let reference = element.attribute(for: name) else {
                    return nil
                }

                return (
                    name: name,
                    reference: reference
                )
            }

            guard values.isEmpty == false else {
                return
            }

            print(padding, "    ", header)

            values.forEach {
                print(padding, "        ", $0.name, $0.reference)
            }
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

    func load(from menuBar: AXUIElement, using options: Options) -> [MenuItem] {
        var items = [MenuItem]()

        guard
            let menuBarItems = menuBar.attribute(for: kAXChildrenAttribute) as? [AXUIElement],
            menuBarItems.count > 0
        else {
            return []
        }

        for (index, item) in menuBarItems.enumerated() {
            guard let name = item.attribute(for: kAXTitleAttribute) as? String else {
                continue
            }

            if name == "Apple" {
                continue
            }

            if options.canIgnorePath(path: [name]) {
                continue
            }

            if let menuRoot = options.specificMenuRoot,
               name.lowercased() != menuRoot.lowercased() {
                continue
            }

            guard let children = item.attribute(for: kAXChildrenAttribute) as? [AXUIElement] else {
                continue
            }

            self.menuItems(
                for: children[0],
                menuItems: &items,
                path: [name],
                pathIndices: [index],
                depth: 1,
                options: options
            )
        }

        return items
    }
}
