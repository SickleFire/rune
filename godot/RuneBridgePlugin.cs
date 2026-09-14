#if TOOLS
using Godot;

[Tool]
public partial class RuneBridgePlugin : EditorPlugin
{
    private const string AutoloadName = "RuneBridge";
    private const string AutoloadPath = "res://addons/rune_bridge/RuneBridge.cs";

    public override void _EnterTree()
    {
        AddAutoloadSingleton(AutoloadName, AutoloadPath);
    }

    public override void _ExitTree()
    {
        RemoveAutoloadSingleton(AutoloadName);
    }
}
#endif