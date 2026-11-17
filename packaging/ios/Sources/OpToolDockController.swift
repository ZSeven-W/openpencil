import UIKit

/// Hosts the Metal editor inside the system navigation container so UIKit
/// can put its toolbar in the iPhone Duo side-bar region.
final class OpToolDockController: UIViewController {
    let playerView: OpPlayerView
    private var selectedSlot: Int32 = -1
    private var selectedShape: UInt32 = 0
    private let shapeIcons: [OpToolDockIcon] = [.rectangle, .ellipse, .polygon, .line]

    init(editorMode: Bool) {
        self.playerView = OpPlayerView(editorMode: editorMode)
        super.init(nibName: nil, bundle: nil)
        playerView.toolDockController = self
    }

    required init?(coder: NSCoder) { return nil }

    override func loadView() {
        view = playerView
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        edgesForExtendedLayout = .all
        extendedLayoutIncludesOpaqueBars = true
        let tools: [(OpToolDockIcon, String, String)] = [
            (.select, "select", "Select"),
            (.rectangle, "shape", "Shape"),
            (.pen, "pen", "Pen"),
            (.text, "text", "Text"),
            (.more, "more", "More"),
        ]
        toolbarItems = tools.enumerated().map { index, tool in
            let image = tool.0.image()
            let item: UIBarButtonItem
            if index == 1 {
                item = UIBarButtonItem(image: image, menu: shapeMenu())
            } else {
                item = UIBarButtonItem(image: image, style: .plain, target: self, action: #selector(pressTool))
            }
            item.tag = index
            item.accessibilityLabel = NSLocalizedString(
                "toolDock.\(tool.1)", value: tool.2, comment: "Native editor toolbar tool"
            )
            return item
        }
    }

    func syncTools(using engine: OpaquePointer) {
        var slot: Int32 = -1
        var shape: UInt32 = 0
        guard op_editor_native_dock_state(engine, &slot, &shape) == OpStatus_Ok,
              let navigationController else { return }

        if shape != selectedShape, shape < shapeIcons.count {
            selectedShape = shape
            let button = toolbarItems?[1]
            button?.image = shapeIcons[Int(shape)].image()
            button?.menu = shapeMenu()
        }
        guard slot != selectedSlot else { return }
        selectedSlot = slot
        for item in toolbarItems ?? [] {
            item.tintColor = item.tag == Int(slot) ? .systemBlue : .label
        }
        navigationController.setToolbarHidden(slot < 0, animated: false)
    }

    private func shapeMenu() -> UIMenu {
        let shapes = [
            ("rectangle", "Rectangle"),
            ("ellipse", "Ellipse"),
            ("polygon", "Polygon"),
            ("line", "Line"),
        ].enumerated().map { index, item in
            shapeAction(UInt32(index), item.0, item.1, icon: shapeIcons[index])
        }
        let pen = shapeAction(6, "pen", "Pen", icon: .pen)
        let insert = [
            shapeAction(4, "icon", "Icon", icon: .icon),
            shapeAction(5, "import", "Import Image or SVG…", icon: .importImage),
        ]
        return UIMenu(children: [
            UIMenu(options: .displayInline, children: shapes + [pen]),
            UIMenu(options: .displayInline, children: insert),
        ])
    }

    private func shapeAction(_ row: UInt32, _ key: String, _ fallback: String, icon: OpToolDockIcon) -> UIAction {
        UIAction(
            title: NSLocalizedString("toolDock.\(key)", value: fallback, comment: "Native shape menu action"),
            image: icon.image(size: 20),
            state: row == selectedShape ? .on : .off
        ) { [weak self] _ in
            guard let self, let engine = self.playerView.host.engine else { return }
            if op_editor_native_shape_press(engine, row) == OpStatus_Ok {
                self.playerView.host.requestImmediateFrame()
            }
        }
    }

    @objc private func pressTool(_ sender: UIBarButtonItem) {
        guard let engine = playerView.host.engine else { return }
        if op_editor_native_dock_press(engine, UInt32(sender.tag)) == OpStatus_Ok {
            playerView.host.requestImmediateFrame()
        }
    }
}
