import QtQuick
import Quickshell
import qs.Ui

BarWidget {
    id: root
    moduleName: "io.github.benryanx.mote"
    implicitWidth: button.implicitWidth
    implicitHeight: button.implicitHeight

    WidgetButton {
        id: button
        anchors.fill: parent
        bar: root.bar
        text: "▦"
        tooltipText: "Mote · Pixel Studio"
        onPressed: function(mouseButton) {
            if (mouseButton === Qt.LeftButton)
                Quickshell.execDetached(["mote"])
        }
    }
}
