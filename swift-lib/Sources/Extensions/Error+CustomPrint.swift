/*
 This source file is part of the Loungy open source project
 
 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License
 
 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

import Foundation

extension Error {
    
    
    /// Custom print error raised from do/catch block, checking if the error type is `MenuBar.Error`
    /// - Parameters:
    ///   - method: function from where the error was raised
    ///   - line: line from where the error was raised
    func printCustomInfo(from method: String = #function, and line: Int = #line) {
        var errorDescription: String = localizedDescription
        
        if let error = self as? MenuBar.Error {
            errorDescription = error.description
        }
        
        Swift.print("[\(method):\(line)] Error: \(errorDescription)")
    }
}
