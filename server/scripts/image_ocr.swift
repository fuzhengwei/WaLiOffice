import AppKit
import Foundation
import Vision

struct OCRResult: Codable {
    let text: String
}

guard CommandLine.arguments.count >= 2 else {
    fputs("{\"text\":\"\"}\n", stderr)
    exit(1)
}

let imagePath = CommandLine.arguments[1]
let imageURL = URL(fileURLWithPath: imagePath)

guard let image = NSImage(contentsOf: imageURL) else {
    fputs("{\"text\":\"\"}\n", stderr)
    exit(2)
}

var proposedRect = CGRect(origin: .zero, size: image.size)
guard let cgImage = image.cgImage(forProposedRect: &proposedRect, context: nil, hints: nil) else {
    fputs("{\"text\":\"\"}\n", stderr)
    exit(3)
}

let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
request.usesLanguageCorrection = true
request.recognitionLanguages = ["zh-Hans", "zh-Hant", "en-US"]

let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])

do {
    try handler.perform([request])
    let observations = request.results ?? []
    let lines = observations.compactMap { observation -> String? in
        guard let candidate = observation.topCandidates(1).first else { return nil }
        let text = candidate.string.trimmingCharacters(in: .whitespacesAndNewlines)
        return text.isEmpty ? nil : text
    }
    let payload = OCRResult(text: lines.joined(separator: "\n"))
    let data = try JSONEncoder().encode(payload)
    if let json = String(data: data, encoding: .utf8) {
        print(json)
    } else {
        print("{\"text\":\"\"}")
    }
} catch {
    fputs("{\"text\":\"\"}\n", stderr)
    exit(4)
}
