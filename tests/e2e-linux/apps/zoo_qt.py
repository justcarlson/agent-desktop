"""Qt widget zoo with independently observable effects."""

import json
import sys
import time
from pathlib import Path


def main(argv):
    from PyQt6.QtCore import Qt, QTimer
    from PyQt6.QtWidgets import (
        QApplication, QCheckBox, QComboBox, QDialog, QFileDialog, QLabel,
        QLineEdit, QListWidget, QListWidgetItem, QMainWindow, QMenu, QMessageBox,
        QPushButton, QRadioButton, QScrollArea, QSlider, QSpinBox, QTabWidget,
        QTextEdit, QToolBar, QToolButton, QVBoxLayout, QWidget, QWidgetAction,
    )
    if len(argv) != 1:
        raise ValueError("expected the JSON-lines log path")
    log = Path(argv[0])
    app = QApplication([sys.argv[0]])
    app.setApplicationName("Zoo Qt")
    resetting = False
    dialogs = []
    window = QMainWindow()
    window.setWindowTitle("Zoo Qt")
    container = QWidget()
    box = QVBoxLayout(container)
    box.setSpacing(3)
    window.setCentralWidget(container)

    def named(widget, name):
        widget.setAccessibleName(name)
        return widget

    def record(name, event, value=None):
        if not resetting:
            with log.open("a") as stream:
                stream.write(json.dumps(dict(widget=name, event=event, value=value,
                                             at=time.monotonic())) + "\n")
                stream.flush()

    def add(widget, name):
        named(widget, name)
        box.addWidget(widget)
        return widget

    def button(name, callback):
        widget = add(QPushButton(name), name)
        widget.clicked.connect(lambda *_: callback())
        return widget

    button("zoo-button", lambda: record("zoo-button", "activate"))
    toggle = add(QPushButton("zoo-toggle"), "zoo-toggle")
    toggle.setCheckable(True)
    check = add(QCheckBox("zoo-check"), "zoo-check")
    radio_a = add(QRadioButton("zoo-radio-a"), "zoo-radio-a")
    radio_b = add(QRadioButton("zoo-radio-b"), "zoo-radio-b")
    radio_a.setChecked(True)
    switch = add(QPushButton("zoo-switch"), "zoo-switch")
    switch.setCheckable(True)
    for widget, name in ((toggle, "zoo-toggle"), (check, "zoo-check"), (switch, "zoo-switch")):
        widget.toggled.connect(lambda value, n=name: record(n, "toggled", value))
    for widget, name in ((radio_a, "zoo-radio-a"), (radio_b, "zoo-radio-b")):
        widget.toggled.connect(lambda value, n=name: value and record(n, "selected", n))
    combo = add(QComboBox(), "zoo-combo")
    combo.addItems(["Alpha", "Bravo", "Charlie"])
    combo.currentTextChanged.connect(lambda value: record("zoo-combo", "changed", value))
    spin = add(QSpinBox(), "zoo-spin")
    spin.setRange(0, 10)
    scale = add(QSlider(Qt.Orientation.Horizontal), "zoo-scale")
    scale.setRange(0, 100)
    for widget, name in ((spin, "zoo-spin"), (scale, "zoo-scale")):
        widget.valueChanged.connect(lambda value, n=name: record(n, "changed", value))
    entry = add(QLineEdit(), "zoo-entry")
    password = add(QLineEdit(), "zoo-password")
    password.setEchoMode(QLineEdit.EchoMode.Password)
    entry.textChanged.connect(lambda value: record("zoo-entry", "changed", value))
    password.textChanged.connect(lambda value: record("zoo-password", "changed", len(value)))
    text = add(QTextEdit(), "zoo-text")
    text.setFixedHeight(60)
    text.textChanged.connect(lambda: record("zoo-text", "changed", text.toPlainText()))
    menu_button = add(QToolButton(), "zoo-menubutton")
    menu_button.setText("zoo-menubutton")
    menu_button.setPopupMode(QToolButton.ToolButtonPopupMode.InstantPopup)
    menu = named(QMenu(menu_button), "zoo-menu")
    item = named(QPushButton("zoo-menu-item"), "zoo-menu-item")
    action = QWidgetAction(menu)
    action.setDefaultWidget(item)
    menu.addAction(action)

    def menu_activated():
        record("zoo-menu-item", "activate")
        menu.close()

    item.clicked.connect(menu_activated)
    menu_button.setMenu(menu)
    link = add(QLabel('<a href="https://example.invalid/">zoo-link</a>'), "zoo-link")
    link.setOpenExternalLinks(False)
    link.setTextInteractionFlags(Qt.TextInteractionFlag.TextBrowserInteraction)
    link.linkActivated.connect(lambda *_: record("zoo-link", "activate"))
    expander = add(QToolButton(), "zoo-expander")
    expander.setText("zoo-expander")
    expander.setCheckable(True)
    expander.setArrowType(Qt.ArrowType.RightArrow)
    expanded = add(QLabel("Expanded content"), "zoo-expander-content")
    expanded.hide()

    def expand(value):
        expanded.setVisible(value)
        expander.setArrowType(Qt.ArrowType.DownArrow if value else Qt.ArrowType.RightArrow)
        record("zoo-expander", "expanded", value)

    expander.toggled.connect(expand)
    tabs = add(QTabWidget(), "zoo-tabs")
    for name in ("zoo-tab-a", "zoo-tab-b"):
        index = tabs.addTab(named(QWidget(), name), name)
        tabs.tabBar().setAccessibleTabName(index, name)
    tabs.currentChanged.connect(lambda i: record(tabs.tabText(i), "page", tabs.tabText(i)))
    rows = add(QListWidget(), "zoo-list")
    rows.setFixedHeight(75)
    for i in range(1, 4):
        name = f"zoo-row-{i}"
        row = QListWidgetItem(name, rows)
        row.setData(Qt.ItemDataRole.AccessibleTextRole, name)
    rows.currentItemChanged.connect(lambda current, previous: current is not None and record(current.text(), "row", current.text()))
    disabled = add(QPushButton("zoo-disabled"), "zoo-disabled")
    disabled.setEnabled(False)
    hidden = add(QPushButton("zoo-hidden"), "zoo-hidden")
    toolbar = named(QToolBar(), "zoo-toolbar")
    toolbar.addWidget(named(QPushButton("zoo-toolbar-hidden"), "zoo-toolbar-hidden"))
    window.addToolBar(toolbar)
    scroll = add(QScrollArea(), "zoo-scroll")
    scroll.setFixedHeight(100)
    scroll.setWidgetResizable(True)
    tall = QWidget()
    tall.setMinimumHeight(2000)
    tall_box = QVBoxLayout(tall)
    tall_box.addStretch()
    offscreen = named(QPushButton("zoo-offscreen"), "zoo-offscreen")
    offscreen.clicked.connect(lambda: record("zoo-offscreen", "activate"))
    tall_box.addWidget(offscreen)
    scroll.setWidget(tall)

    def show_dialog(dialog):
        dialogs.append(dialog)
        dialog.finished.connect(lambda *_: dialogs.remove(dialog))
        dialog.show()

    def open_dialog():
        dialog = QDialog(window)
        dialog.setWindowTitle("Zoo Dialog")
        dialog.setModal(True)
        layout = QVBoxLayout(dialog)
        field = named(QLineEdit(), "zoo-dialog-entry")
        field.textChanged.connect(lambda value: record("zoo-dialog-entry", "changed", value))
        layout.addWidget(field)
        ok = named(QPushButton("OK"), "zoo-dialog-ok")
        cancel = named(QPushButton("Cancel"), "zoo-dialog-cancel")
        layout.addWidget(ok)
        layout.addWidget(cancel)
        ok.clicked.connect(lambda: record("zoo-dialog-ok", "activate"))
        ok.clicked.connect(dialog.accept)
        cancel.clicked.connect(dialog.reject)
        show_dialog(dialog)

    def open_chooser():
        dialog = QFileDialog(window)
        dialog.setOption(QFileDialog.Option.DontUseNativeDialog, True)
        dialog.setWindowTitle("Zoo Save")
        dialog.setAcceptMode(QFileDialog.AcceptMode.AcceptSave)
        dialog.setModal(True)
        dialog.fileSelected.connect(lambda path: record("zoo-open-chooser", "saved", path))
        show_dialog(dialog)

    def open_message():
        dialog = QMessageBox(window)
        dialog.setWindowTitle("Zoo Message")
        dialog.setText("Zoo Message")
        dialog.setStandardButtons(QMessageBox.StandardButton.Ok)
        named(dialog.button(QMessageBox.StandardButton.Ok), "OK")
        show_dialog(dialog)

    def open_toplevel():
        top = named(QPushButton("zoo-top-button"), "zoo-top-button")
        top.setWindowTitle("Zoo Top Button")
        top.clicked.connect(lambda: record("zoo-top-button", "activate"))
        dialogs.append(top)
        top.show()

    button("zoo-open-dialog", open_dialog)
    button("zoo-open-chooser", open_chooser)
    button("zoo-open-message", open_message)
    button("zoo-open-toplevel-button", open_toplevel)

    def control():
        nonlocal resetting
        reset = Path(str(log) + ".reset")
        if reset.exists():
            resetting = True
            for dialog in list(dialogs):
                dialog.close()
            dialogs.clear()
            menu.close()
            for widget in (toggle, check, switch, expander):
                widget.setChecked(False)
            radio_a.setChecked(True)
            combo.setCurrentIndex(0)
            spin.setValue(0)
            scale.setValue(0)
            entry.clear()
            password.clear()
            text.clear()
            tabs.setCurrentIndex(0)
            rows.clearSelection()
            rows.setCurrentRow(-1)
            scroll.verticalScrollBar().setValue(0)
            scroll.horizontalScrollBar().setValue(0)
            resetting = False
            reset.rename(Path(str(log) + ".reset-consumed"))
            record("zoo", "reset")

    timer = QTimer(window)
    timer.timeout.connect(control)
    timer.start(20)
    window.resize(600, 1000)
    window.show()
    hidden.hide()
    toolbar.hide()
    return app.exec()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
