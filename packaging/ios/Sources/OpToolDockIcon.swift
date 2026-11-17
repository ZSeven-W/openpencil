import UIKit

/// Template glyphs in the OpenPencil canvas toolbar's outline style.
/// UIKit tints these images and lays them out in either toolbar direction.
enum OpToolDockIcon {
    case select, rectangle, ellipse, polygon, line, pen, text, more, icon, importImage

    func image(size: CGFloat = 24) -> UIImage {
        UIGraphicsImageRenderer(size: CGSize(width: size, height: size)).image { renderer in
            let context = renderer.cgContext
            let scale = size / 24
            context.scaleBy(x: scale, y: scale)
            context.setLineWidth(1.8)
            context.setLineCap(.round)
            context.setLineJoin(.round)
            context.setStrokeColor(UIColor.black.cgColor)
            context.setFillColor(UIColor.black.cgColor)

            func stroke(_ points: [(CGFloat, CGFloat)], closed: Bool = false) {
                guard let first = points.first else { return }
                context.move(to: CGPoint(x: first.0, y: first.1))
                for point in points.dropFirst() {
                    context.addLine(to: CGPoint(x: point.0, y: point.1))
                }
                if closed { context.closePath() }
                context.strokePath()
            }

            func roundedRect(_ rect: CGRect, radius: CGFloat) {
                context.addPath(UIBezierPath(roundedRect: rect, cornerRadius: radius).cgPath)
                context.strokePath()
            }

            switch self {
            case .select:
                stroke([(4, 4), (20, 10.5), (13.8, 12.2), (12.2, 13.8), (10.5, 20)], closed: true)
            case .rectangle:
                roundedRect(CGRect(x: 3.5, y: 3.5, width: 17, height: 17), radius: 2.5)
            case .ellipse:
                context.strokeEllipse(in: CGRect(x: 3.5, y: 3.5, width: 17, height: 17))
            case .polygon:
                stroke([(12, 4), (21, 19.5), (3, 19.5)], closed: true)
            case .line:
                stroke([(4.5, 19.5), (19.5, 4.5)])
            case .pen:
                stroke([(13, 17.3), (6.3, 15.8), (3, 3), (15.8, 6.3), (17.3, 13)])
                stroke([(12.5, 18), (18, 12.5), (21, 15.5), (15.5, 21)], closed: true)
                stroke([(3, 3), (9, 9)])
                context.strokeEllipse(in: CGRect(x: 8.3, y: 8.3, width: 3.4, height: 3.4))
            case .text:
                stroke([(4.5, 7), (4.5, 4.5), (19.5, 4.5), (19.5, 7)])
                stroke([(12, 4.5), (12, 19.5)])
                stroke([(8.5, 19.5), (15.5, 19.5)])
            case .more:
                for x: CGFloat in [5, 12, 19] {
                    context.fillEllipse(in: CGRect(x: x - 1.35, y: 10.65, width: 2.7, height: 2.7))
                }
            case .icon:
                stroke([(11, 3), (13, 9), (19, 11), (13, 13), (11, 19), (9, 13), (3, 11), (9, 9)], closed: true)
                stroke([(19, 3), (19, 7)])
                stroke([(17, 5), (21, 5)])
            case .importImage:
                roundedRect(CGRect(x: 3.5, y: 4.5, width: 17, height: 15), radius: 2)
                context.strokeEllipse(in: CGRect(x: 7, y: 8, width: 3, height: 3))
                stroke([(5, 18.5), (11, 13), (13.5, 15.5), (16.5, 12.5), (20, 16)])
            }
        }.withRenderingMode(.alwaysTemplate)
    }
}
