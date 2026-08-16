import UIKit

/// Shared visual language for the native SSO screens, matching the ZSeven
/// web sign-in design: labeled boxed inputs with leading icons, a blue→purple
/// gradient primary button, hairline-bordered provider icon cards, and blue
/// text links.
enum AuthTheme {
    static let accent = UIColor(red: 0.18, green: 0.42, blue: 1.0, alpha: 1)
    static let gradientStart = UIColor(red: 0.18, green: 0.42, blue: 1.0, alpha: 1).cgColor
    static let gradientEnd = UIColor(red: 0.48, green: 0.36, blue: 1.0, alpha: 1).cgColor
    static let fieldCorner: CGFloat = 12
    static let fieldHeight: CGFloat = 52

    static func fieldBorder() -> UIColor {
        UIColor { trait in
            trait.userInterfaceStyle == .dark
                ? UIColor(white: 1, alpha: 0.18)
                : UIColor(white: 0, alpha: 0.12)
        }
    }

    /// Section label above an input ("邮箱地址").
    static func makeFieldLabel(_ text: String) -> UILabel {
        let label = UILabel()
        label.text = text
        label.font = .systemFont(ofSize: 14, weight: .semibold)
        label.textColor = .label
        return label
    }

    /// Bundled lucide stroke icon redrawn at a fixed point size; template
    /// rendering so it follows the caller's tint color.
    static func lucideIcon(_ assetName: String, pointSize: CGFloat) -> UIImage? {
        UIImage(named: assetName)?
            .resized(to: CGSize(width: pointSize, height: pointSize))
            .withRenderingMode(.alwaysTemplate)
    }

    /// Boxed input row: rounded hairline border, leading lucide icon, the
    /// text field itself, and an optional trailing accessory (eye toggle /
    /// send code). Returns the container; the caller keeps the field
    /// reference.
    static func makeBoxedField(
        _ field: UITextField,
        placeholder: String,
        iconAsset: String,
        trailing: UIView? = nil
    ) -> UIView {
        let container = UIView()
        container.layer.cornerRadius = fieldCorner
        container.layer.borderWidth = 1
        container.layer.borderColor = fieldBorder().cgColor
        container.backgroundColor = .secondarySystemBackground
        container.heightAnchor.constraint(equalToConstant: fieldHeight).isActive = true

        let icon = UIImageView(image: lucideIcon(iconAsset, pointSize: 18))
        icon.tintColor = .tertiaryLabel
        icon.contentMode = .scaleAspectFit
        icon.translatesAutoresizingMaskIntoConstraints = false

        field.placeholder = placeholder
        field.font = .systemFont(ofSize: 16)
        field.borderStyle = .none
        field.translatesAutoresizingMaskIntoConstraints = false

        container.addSubview(icon)
        container.addSubview(field)
        var constraints = [
            icon.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 14),
            icon.centerYAnchor.constraint(equalTo: container.centerYAnchor),
            icon.widthAnchor.constraint(equalToConstant: 18),
            icon.heightAnchor.constraint(equalToConstant: 18),
            field.leadingAnchor.constraint(equalTo: icon.trailingAnchor, constant: 10),
            field.centerYAnchor.constraint(equalTo: container.centerYAnchor),
        ]
        if let trailing {
            trailing.translatesAutoresizingMaskIntoConstraints = false
            trailing.setContentHuggingPriority(.required, for: .horizontal)
            trailing.setContentCompressionResistancePriority(.required, for: .horizontal)
            container.addSubview(trailing)
            constraints.append(contentsOf: [
                trailing.trailingAnchor.constraint(
                    equalTo: container.trailingAnchor,
                    constant: -12
                ),
                trailing.centerYAnchor.constraint(equalTo: container.centerYAnchor),
                field.trailingAnchor.constraint(
                    equalTo: trailing.leadingAnchor,
                    constant: -8
                ),
            ])
        } else {
            constraints.append(
                field.trailingAnchor.constraint(
                    equalTo: container.trailingAnchor,
                    constant: -14
                )
            )
        }
        NSLayoutConstraint.activate(constraints)
        return container
    }

    /// Full-width primary button with the brand gradient.
    static func makePrimaryButton(title: String, target: Any?, action: Selector) -> UIButton {
        let button = GradientButton(type: .system)
        button.setTitle(title, for: .normal)
        button.setTitleColor(.white, for: .normal)
        button.titleLabel?.font = .systemFont(ofSize: 17, weight: .semibold)
        button.layer.cornerRadius = fieldCorner
        button.clipsToBounds = true
        button.heightAnchor.constraint(equalToConstant: fieldHeight).isActive = true
        button.addTarget(target, action: action, for: .touchUpInside)
        return button
    }

    /// Small blue text link ("忘记密码?", "立即注册").
    static func makeLink(title: String, target: Any?, action: Selector) -> UIButton {
        var config = UIButton.Configuration.plain()
        config.title = title
        config.baseForegroundColor = accent
        config.contentInsets = .zero
        config.titleTextAttributesTransformer = UIConfigurationTextAttributesTransformer {
            var attrs = $0
            attrs.font = UIFont.systemFont(ofSize: 14, weight: .medium)
            return attrs
        }
        let button = UIButton(configuration: config)
        button.addTarget(target, action: action, for: .touchUpInside)
        return button
    }

    /// "── 或使用以下方式继续 ──" divider row.
    static func makeDivider(text: String) -> UIView {
        let container = UIStackView()
        container.axis = .horizontal
        container.alignment = .center
        container.spacing = 12
        let label = UILabel()
        label.text = text
        label.font = .systemFont(ofSize: 13)
        label.textColor = .secondaryLabel
        label.setContentHuggingPriority(.required, for: .horizontal)
        let left = UIView()
        let right = UIView()
        for line in [left, right] {
            line.backgroundColor = fieldBorder()
            line.heightAnchor.constraint(equalToConstant: 1).isActive = true
        }
        container.addArrangedSubview(left)
        container.addArrangedSubview(label)
        container.addArrangedSubview(right)
        right.widthAnchor.constraint(equalTo: left.widthAnchor).isActive = true
        return container
    }

    private static let monochromeProviderAssets: Set<String> = [
        "Provider-apple", "Provider-github", "Provider-douyin",
    ]

    /// Rounded bordered card holding one provider brand icon.
    static func makeProviderCard(assetName: String, target: Any?, action: Selector) -> UIButton {
        let button = UIButton(type: .system)
        button.layer.cornerRadius = 14
        button.layer.borderWidth = 1
        button.layer.borderColor = fieldBorder().cgColor
        // Adaptive card: white in light mode, elevated dark in dark mode —
        // the monochrome brand icons carry dark-appearance white variants in
        // the asset catalog so they stay legible on both.
        button.backgroundColor = UIColor { trait in
            trait.userInterfaceStyle == .dark
                ? UIColor(white: 1, alpha: 0.08)
                : .white
        }
        button.widthAnchor.constraint(equalToConstant: 56).isActive = true
        button.heightAnchor.constraint(equalToConstant: 56).isActive = true
        if let image = UIImage(named: assetName) {
            let sized = image.resized(to: CGSize(width: 30, height: 30))
            // Monochrome brand marks follow the label color so they stay
            // legible on the adaptive card; colored marks keep their brand
            // colors as-is.
            if Self.monochromeProviderAssets.contains(assetName) {
                button.setImage(sized.withRenderingMode(.alwaysTemplate), for: .normal)
            } else {
                button.setImage(sized, for: .normal)
            }
        } else {
            button.setImage(lucideIcon("Icon-globe", pointSize: 24), for: .normal)
        }
        button.tintColor = .label
        button.addTarget(target, action: action, for: .touchUpInside)
        return button
    }

    /// Centered footer prompt + link ("还没有账号? 立即注册").
    static func makeFooterPrompt(
        prompt: String,
        linkTitle: String,
        target: Any?,
        action: Selector
    ) -> UIView {
        let row = UIStackView()
        row.axis = .horizontal
        row.alignment = .center
        row.spacing = 4
        let label = UILabel()
        label.text = prompt
        label.font = .systemFont(ofSize: 14)
        label.textColor = .secondaryLabel
        row.addArrangedSubview(label)
        row.addArrangedSubview(makeLink(title: linkTitle, target: target, action: action))
        let container = UIView()
        row.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(row)
        NSLayoutConstraint.activate([
            row.centerXAnchor.constraint(equalTo: container.centerXAnchor),
            row.topAnchor.constraint(equalTo: container.topAnchor),
            row.bottomAnchor.constraint(equalTo: container.bottomAnchor),
        ])
        return container
    }
}

/// Renders the brand gradient behind the title; kept a plain subclass so the
/// gradient tracks bounds changes.
final class GradientButton: UIButton {
    private let gradient = CAGradientLayer()

    override init(frame: CGRect) {
        super.init(frame: frame)
        installGradient()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        installGradient()
    }

    private func installGradient() {
        gradient.colors = [AuthTheme.gradientStart, AuthTheme.gradientEnd]
        gradient.startPoint = CGPoint(x: 0, y: 0.5)
        gradient.endPoint = CGPoint(x: 1, y: 0.5)
        layer.insertSublayer(gradient, at: 0)
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        gradient.frame = bounds
    }
}

extension UIImage {
    /// Fitted redraw used for fixed-size button imagery.
    func resized(to size: CGSize) -> UIImage {
        UIGraphicsImageRenderer(size: size).image { _ in
            draw(in: CGRect(origin: .zero, size: size))
        }.withRenderingMode(.alwaysOriginal)
    }
}
