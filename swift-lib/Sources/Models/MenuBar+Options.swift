/*
 This source file is part of the Loungy open source project
 
 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License
 
 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

extension MenuBar {
    struct Options {
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
}
