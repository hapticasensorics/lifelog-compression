import AVFoundation
import CoreGraphics
import CoreMedia
import Foundation
import ImageIO
import UniformTypeIdentifiers

struct Options {
    let input: URL
    let outputDir: URL
    let intervalSeconds: Double
    let toleranceBeforeMs: Int
    let toleranceAfterMs: Int
    let canvasWidth: Int
    let canvasHeight: Int
    let jpegQuality: Double
}

struct NativeSourceVideoMetadata: Codable {
    let fileSizeBytes: UInt64
    let containerFormat: String
    let videoCodec: String
    let durationMs: UInt64
    let width: Int
    let height: Int
    let displayWidth: Int
    let displayHeight: Int
    let aspectRatio: Double
    let avgFrameRate: String
    let rotationDegrees: Int
    let creationTime: String?
    let timecode: String?
    let hasAudio: Bool
}

struct NativeFrameRecord: Codable {
    let frameRelpath: String
    let requestedTsMs: UInt64
    let actualTsMs: UInt64
}

struct Payload: Codable {
    let sourceVideo: NativeSourceVideoMetadata
    let frames: [NativeFrameRecord]
}

enum ExtractError: Error, CustomStringConvertible {
    case usage
    case badValue(String)
    case missingVideoTrack
    case createContextFailed
    case imageWriteFailed(String)

    var description: String {
        switch self {
        case .usage:
            return "usage: apple-native-extract --input <video> --output-dir <dir> --interval-seconds 1 --tolerance-before-ms 500 --tolerance-after-ms 500 --canvas-width 1920 --canvas-height 1080 --jpeg-quality 0.75"
        case let .badValue(value):
            return "bad value: \(value)"
        case .missingVideoTrack:
            return "input asset is missing a video track"
        case .createContextFailed:
            return "failed to create drawing context"
        case let .imageWriteFailed(path):
            return "failed to write jpeg: \(path)"
        }
    }
}

func parseArgs() throws -> Options {
    var input: String?
    var outputDir: String?
    var intervalSeconds = 1.0
    var toleranceBeforeMs = 500
    var toleranceAfterMs = 500
    var canvasWidth = 1920
    var canvasHeight = 1080
    var jpegQuality = 0.75

    var iterator = CommandLine.arguments.dropFirst().makeIterator()
    while let arg = iterator.next() {
        switch arg {
        case "--input":
            input = iterator.next()
        case "--output-dir":
            outputDir = iterator.next()
        case "--interval-seconds":
            guard let value = iterator.next(), let parsed = Double(value) else {
                throw ExtractError.badValue(arg)
            }
            intervalSeconds = parsed
        case "--tolerance-before-ms":
            guard let value = iterator.next(), let parsed = Int(value) else {
                throw ExtractError.badValue(arg)
            }
            toleranceBeforeMs = parsed
        case "--tolerance-after-ms":
            guard let value = iterator.next(), let parsed = Int(value) else {
                throw ExtractError.badValue(arg)
            }
            toleranceAfterMs = parsed
        case "--canvas-width":
            guard let value = iterator.next(), let parsed = Int(value) else {
                throw ExtractError.badValue(arg)
            }
            canvasWidth = parsed
        case "--canvas-height":
            guard let value = iterator.next(), let parsed = Int(value) else {
                throw ExtractError.badValue(arg)
            }
            canvasHeight = parsed
        case "--jpeg-quality":
            guard let value = iterator.next(), let parsed = Double(value) else {
                throw ExtractError.badValue(arg)
            }
            jpegQuality = parsed
        default:
            throw ExtractError.usage
        }
    }

    guard let input, let outputDir else {
        throw ExtractError.usage
    }

    return Options(
        input: URL(fileURLWithPath: input),
        outputDir: URL(fileURLWithPath: outputDir),
        intervalSeconds: intervalSeconds,
        toleranceBeforeMs: toleranceBeforeMs,
        toleranceAfterMs: toleranceAfterMs,
        canvasWidth: canvasWidth,
        canvasHeight: canvasHeight,
        jpegQuality: jpegQuality
    )
}

func rotationDegrees(for transform: CGAffineTransform) -> Int {
    let epsilon = 0.0001
    func near(_ lhs: CGFloat, _ rhs: CGFloat) -> Bool {
        abs(lhs - rhs) < epsilon
    }

    if near(transform.a, 0), near(transform.b, 1), near(transform.c, -1), near(transform.d, 0) {
        return 90
    }
    if near(transform.a, 0), near(transform.b, -1), near(transform.c, 1), near(transform.d, 0) {
        return 270
    }
    if near(transform.a, -1), near(transform.b, 0), near(transform.c, 0), near(transform.d, -1) {
        return 180
    }
    return 0
}

func fourCCString(_ value: FourCharCode) -> String {
    let chars = [
        Character(UnicodeScalar((value >> 24) & 255)!),
        Character(UnicodeScalar((value >> 16) & 255)!),
        Character(UnicodeScalar((value >> 8) & 255)!),
        Character(UnicodeScalar(value & 255)!),
    ]
    return String(chars)
}

func codecName(from track: AVAssetTrack) -> String {
    guard let description = track.formatDescriptions.first else {
        return "unknown"
    }
    let mediaType = CMFormatDescriptionGetMediaSubType(description as! CMFormatDescription)
    return fourCCString(mediaType)
}

func creationTimeString(from asset: AVAsset) -> String? {
    let metadata = asset.commonMetadata.first { item in
        item.commonKey?.rawValue == "creationDate"
    }
    return metadata?.stringValue
}

func timecodeString(from track: AVAssetTrack) -> String? {
    track.metadata.first { item in
        item.commonKey?.rawValue == "timecode"
    }?.stringValue
}

func paddedImage(_ image: CGImage, canvasWidth: Int, canvasHeight: Int) throws -> CGImage {
    let colorSpace = CGColorSpaceCreateDeviceRGB()
    let bitmapInfo = CGImageAlphaInfo.noneSkipLast.rawValue

    guard let context = CGContext(
        data: nil,
        width: canvasWidth,
        height: canvasHeight,
        bitsPerComponent: 8,
        bytesPerRow: 0,
        space: colorSpace,
        bitmapInfo: bitmapInfo
    ) else {
        throw ExtractError.createContextFailed
    }

    context.setFillColor(CGColor(red: 0, green: 0, blue: 0, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: canvasWidth, height: canvasHeight))

    let scale = min(
        Double(canvasWidth) / Double(image.width),
        Double(canvasHeight) / Double(image.height)
    )
    let targetWidth = Double(image.width) * scale
    let targetHeight = Double(image.height) * scale
    let x = (Double(canvasWidth) - targetWidth) / 2.0
    let y = (Double(canvasHeight) - targetHeight) / 2.0

    context.draw(image, in: CGRect(x: x, y: y, width: targetWidth, height: targetHeight))

    guard let output = context.makeImage() else {
        throw ExtractError.createContextFailed
    }
    return output
}

func writeJPEG(_ image: CGImage, to url: URL, quality: Double) throws {
    guard let destination = CGImageDestinationCreateWithURL(
        url as CFURL,
        UTType.jpeg.identifier as CFString,
        1,
        nil
    ) else {
        throw ExtractError.imageWriteFailed(url.path)
    }
    let properties: NSDictionary = [
        kCGImageDestinationLossyCompressionQuality: quality
    ]
    CGImageDestinationAddImage(destination, image, properties)
    if !CGImageDestinationFinalize(destination) {
        throw ExtractError.imageWriteFailed(url.path)
    }
}

func run() throws {
    let options = try parseArgs()
    let fileManager = FileManager.default
    let framesDir = options.outputDir.appendingPathComponent("frames", isDirectory: true)
    try fileManager.createDirectory(at: framesDir, withIntermediateDirectories: true)

    let asset = AVURLAsset(url: options.input)
    guard let track = asset.tracks(withMediaType: .video).first else {
        throw ExtractError.missingVideoTrack
    }

    let transform = track.preferredTransform
    let naturalSize = track.naturalSize
    let displayWidth = Int(abs(naturalSize.applying(transform).width).rounded())
    let displayHeight = Int(abs(naturalSize.applying(transform).height).rounded())
    let width = Int(naturalSize.width.rounded())
    let height = Int(naturalSize.height.rounded())
    let fileAttributes = try fileManager.attributesOfItem(atPath: options.input.path)
    let fileSizeBytes = (fileAttributes[.size] as? NSNumber)?.uint64Value ?? 0
    let durationMs = UInt64((asset.duration.seconds * 1000.0).rounded())
    let hasAudio = !asset.tracks(withMediaType: .audio).isEmpty
    let avgFrameRate = String(format: "%.3f", track.nominalFrameRate)

    let generator = AVAssetImageGenerator(asset: asset)
    generator.appliesPreferredTrackTransform = true
    generator.maximumSize = CGSize(width: options.canvasWidth, height: options.canvasHeight)
    generator.requestedTimeToleranceBefore = CMTime(
        seconds: Double(options.toleranceBeforeMs) / 1000.0,
        preferredTimescale: 600
    )
    generator.requestedTimeToleranceAfter = CMTime(
        seconds: Double(options.toleranceAfterMs) / 1000.0,
        preferredTimescale: 600
    )

    var frames: [NativeFrameRecord] = []
    var index = 0
    var requestedSeconds = 0.0
    while requestedSeconds < asset.duration.seconds {
        let requested = CMTime(seconds: requestedSeconds, preferredTimescale: 600)
        var actual = CMTime.zero
        let image = try generator.copyCGImage(at: requested, actualTime: &actual)
        let padded = try paddedImage(
            image,
            canvasWidth: options.canvasWidth,
            canvasHeight: options.canvasHeight
        )
        let filename = String(format: "%08d.jpg", index + 1)
        let frameURL = framesDir.appendingPathComponent(filename)
        try writeJPEG(padded, to: frameURL, quality: options.jpegQuality)
        frames.append(
            NativeFrameRecord(
                frameRelpath: "frames/\(filename)",
                requestedTsMs: UInt64((requestedSeconds * 1000.0).rounded()),
                actualTsMs: UInt64((actual.seconds * 1000.0).rounded())
            )
        )
        index += 1
        requestedSeconds += options.intervalSeconds
    }

    let payload = Payload(
        sourceVideo: NativeSourceVideoMetadata(
            fileSizeBytes: fileSizeBytes,
            containerFormat: options.input.pathExtension.lowercased(),
            videoCodec: codecName(from: track),
            durationMs: durationMs,
            width: width,
            height: height,
            displayWidth: displayWidth,
            displayHeight: displayHeight,
            aspectRatio: displayHeight > 0 ? Double(displayWidth) / Double(displayHeight) : 0,
            avgFrameRate: avgFrameRate,
            rotationDegrees: rotationDegrees(for: transform),
            creationTime: creationTimeString(from: asset),
            timecode: timecodeString(from: track),
            hasAudio: hasAudio
        ),
        frames: frames
    )

    let encoder = JSONEncoder()
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    let data = try encoder.encode(payload)
    FileHandle.standardOutput.write(data)
    FileHandle.standardOutput.write("\n".data(using: .utf8)!)
}

do {
    try run()
} catch {
    fputs("\(error)\n", stderr)
    exit(1)
}
