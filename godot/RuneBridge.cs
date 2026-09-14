using Godot;
using System;
using System.IO;
using System.Net;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

[GlobalClass]
public partial class RuneBridge : Node
{
    private HttpListener _listener;
    private const string Port = "8089";

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true
    };

    public override void _Ready()
    {
        try
        {
            _listener = new HttpListener();
            _listener.Prefixes.Add($"http://localhost:{Port}/");
            _listener.Start();
            _listener.BeginGetContext(OnRequest, null);
            GD.Print($"[RuneBridge] Listening on http://localhost:{Port}/");
        }
        catch (Exception ex)
        {
            GD.PrintErr($"[RuneBridge] Failed to start on port {Port}: {ex.Message}");
        }
    }

    public override void _ExitTree()
    {
        try
        {
            _listener?.Stop();
            _listener?.Close();
            GD.Print("[RuneBridge] Stopped.");
        }
        catch (Exception ex)
        {
            GD.PrintErr($"[RuneBridge] Error stopping: {ex.Message}");
        }
    }

    private void OnRequest(IAsyncResult result)
    {
        try
        {
            HttpListenerContext context = _listener.EndGetContext(result);
            _listener.BeginGetContext(OnRequest, null);
            Callable.From(() => HandleContext(context)).CallDeferred();
        }
        catch
        {
            // Listener was stopped or disposed.
        }
    }

    private void HandleContext(HttpListenerContext ctx)
    {
        string path   = ctx.Request.Url.AbsolutePath;
        string method = ctx.Request.HttpMethod;

        try
        {
            switch (path)
            {
                case "/scene/inspect" when method == "GET":
                    Send(ctx, GetSceneHierarchyJson(), HttpStatusCode.OK);
                    break;

                case "/node/inspect-properties" when method == "GET":
                {
                    string nodePath = ctx.Request.QueryString["nodePath"];
                    if (string.IsNullOrEmpty(nodePath))
                    {
                        Send(ctx, Err("Missing 'nodePath' query parameter"), HttpStatusCode.BadRequest);
                        return;
                    }
                    Send(ctx, InspectNodeProperties(nodePath), HttpStatusCode.OK);
                    break;
                }

                case "/node/set-property" when method == "POST":
                {
                    bool ok = SetNodeProperty(ReadBody(ctx));
                    Send(ctx,
                        ok ? "{\"status\":\"success\"}" : Err("Failed to set property"),
                        ok ? HttpStatusCode.OK : HttpStatusCode.BadRequest);
                    break;
                }

                case "/node/create" when method == "POST":
                    Send(ctx, CreateNode(ReadBody(ctx)), HttpStatusCode.OK);
                    break;

                case "/node/destroy" when method == "POST":
                    Send(ctx, DestroyNode(ReadBody(ctx)), HttpStatusCode.OK);
                    break;

                case "/node/call-method" when method == "POST":
                    Send(ctx, CallNodeMethod(ReadBody(ctx)), HttpStatusCode.OK);
                    break;

                default:
                    Send(ctx, Err("Endpoint not found"), HttpStatusCode.NotFound);
                    break;
            }
        }
        catch (Exception ex)
        {
            Send(ctx, Err(ex.Message), HttpStatusCode.InternalServerError);
        }
    }

    private string GetSceneHierarchyJson()
    {
        var scene = GetTree()?.CurrentScene;
        if (scene == null)
            return "{\"sceneName\":null,\"rootNodes\":[]}";

        var sb = new StringBuilder();
        sb.Append("{\"sceneName\":\"").Append(Esc(scene.Name)).Append("\",\"rootNodes\":[");
        SerializeNode(scene, sb);
        sb.Append("]}");
        return sb.ToString();
    }

    private void SerializeNode(Node node, StringBuilder sb)
    {
        sb.Append("{\"name\":\"").Append(Esc(node.Name))
          .Append("\",\"type\":\"").Append(Esc(node.GetType().Name))
          .Append("\",\"path\":\"").Append(Esc(node.GetPath().ToString()))
          .Append("\",\"children\":[");

        int count = node.GetChildCount();
        for (int i = 0; i < count; i++)
        {
            SerializeNode(node.GetChild(i), sb);
            if (i < count - 1) sb.Append(',');
        }

        sb.Append("]}");
    }

    private string InspectNodeProperties(string nodePath)
    {
        var node = ResolveNode(nodePath);
        if (node == null) return Err("Node not found");

        var sb = new StringBuilder();
        sb.Append("{\"nodeName\":\"").Append(Esc(node.Name))
          .Append("\",\"nodeType\":\"").Append(Esc(node.GetType().Name))
          .Append("\",\"nodePath\":\"").Append(Esc(node.GetPath().ToString()))
          .Append("\",\"properties\":[");

        bool first = true;
        foreach (Godot.Collections.Dictionary prop in node.GetPropertyList())
        {
            if (!first) sb.Append(',');
            first = false;

            string name   = prop["name"].AsString();
            int    type   = prop["type"].AsInt32();
            string valStr = node.Get(name).ToString() ?? "null";

            sb.Append("{\"name\":\"").Append(Esc(name))
              .Append("\",\"type\":").Append(type)
              .Append(",\"value\":\"").Append(Esc(valStr)).Append("\"}");
        }

        sb.Append("]}");
        return sb.ToString();
    }

    private bool SetNodeProperty(string body)
    {
        var payload = Deserialize<SetPropertyPayload>(body);
        var node = ResolveNode(payload.NodePath);
        if (node == null) return false;

        try
        {
            node.Set(payload.PropertyName, Variant.From(payload.Value));
            return true;
        }
        catch (Exception ex)
        {
            GD.PrintErr($"[RuneBridge] SetProperty failed: {ex.Message}");
            return false;
        }
    }

    private string CreateNode(string body)
    {
        var payload = Deserialize<CreateNodePayload>(body);
        var parent  = ResolveNode(payload.ParentPath) ?? GetTree()?.CurrentScene;
        if (parent == null) return Err("Parent node not found");

        // Use ClassDB so any Godot built-in type works, not just a hand-rolled switch.
        Node newNode;
        if (!string.IsNullOrEmpty(payload.NodeType) && ClassDB.ClassExists(payload.NodeType))
        {
            var obj = ClassDB.Instantiate(payload.NodeType);
            if (obj.Obj is not Node typedNode)
                return Err($"'{payload.NodeType}' is not a Node subclass");
            newNode = typedNode;
        }
        else
        {
            newNode = new Node();
        }

        newNode.Name     = string.IsNullOrEmpty(payload.NodeName) ? "NewNode" : payload.NodeName;
        parent.AddChild(newNode);
        newNode.Owner    = GetTree()?.CurrentScene;

        return $"{{\"status\":\"success\",\"nodeName\":\"{Esc(newNode.Name)}\",\"path\":\"{Esc(newNode.GetPath().ToString())}\"}}";
    }

    private string DestroyNode(string body)
    {
        var payload = Deserialize<DestroyNodePayload>(body);
        var node    = ResolveNode(payload.NodePath);
        if (node == null) return Err("Node not found");

        string name = node.Name;
        node.QueueFree();
        return $"{{\"status\":\"success\",\"destroyed\":\"{Esc(name)}\"}}";
    }

    private string CallNodeMethod(string body)
    {
        var payload = Deserialize<CallMethodPayload>(body);
        var node    = ResolveNode(payload.NodePath);
        if (node == null) return Err("Node not found");

        try
        {
            // args are passed as strings; Godot will coerce where possible.
            var args = payload.Args is { Length: > 0 }
                ? Array.ConvertAll(payload.Args, a => Variant.From(a))
                : Array.Empty<Variant>();

            Variant result = node.Call(payload.MethodName, args);
            return $"{{\"status\":\"success\",\"result\":\"{Esc(result.ToString())}\"}}";
        }
        catch (Exception ex)
        {
            return Err($"Method call failed: {ex.Message}");
        }
    }

    private struct SetPropertyPayload
    {
        [JsonPropertyName("nodePath")]     public string NodePath     { get; set; }
        [JsonPropertyName("propertyName")] public string PropertyName { get; set; }
        [JsonPropertyName("value")]        public string Value        { get; set; }
    }

    private struct CreateNodePayload
    {
        [JsonPropertyName("parentPath")] public string ParentPath { get; set; }
        [JsonPropertyName("nodeName")]   public string NodeName   { get; set; }
        [JsonPropertyName("nodeType")]   public string NodeType   { get; set; }
    }

    private struct DestroyNodePayload
    {
        [JsonPropertyName("nodePath")] public string NodePath { get; set; }
    }

    private struct CallMethodPayload
    {
        [JsonPropertyName("nodePath")]   public string   NodePath   { get; set; }
        [JsonPropertyName("methodName")] public string   MethodName { get; set; }
        [JsonPropertyName("args")]       public string[] Args       { get; set; }
    }

    private Node ResolveNode(string path)
    {
        if (string.IsNullOrEmpty(path)) return null;
        return GetNodeOrNull(path) ?? GetTree()?.Root.FindChild(path, true, false);
    }

    private T Deserialize<T>(string json) =>
        JsonSerializer.Deserialize<T>(json, JsonOptions);

    private static string Esc(string s)
    {
        if (string.IsNullOrEmpty(s)) return "";
        return s.Replace("\\", "\\\\")
                .Replace("\"", "\\\"")
                .Replace("\n", "\\n")
                .Replace("\r", "\\r")
                .Replace("\t", "\\t");
    }

    private static string Err(string msg) =>
        $"{{\"error\":\"{Esc(msg)}\"}}";

    private static string ReadBody(HttpListenerContext ctx)
    {
        using var reader = new StreamReader(ctx.Request.InputStream, ctx.Request.ContentEncoding);
        return reader.ReadToEnd();
    }

    private static void Send(HttpListenerContext ctx, string json, HttpStatusCode status)
    {
        try
        {
            byte[] buf = Encoding.UTF8.GetBytes(json ?? "{}");
            ctx.Response.StatusCode       = (int)status;
            ctx.Response.ContentType      = "application/json";
            ctx.Response.ContentEncoding  = Encoding.UTF8;
            ctx.Response.ContentLength64  = buf.Length;
            ctx.Response.OutputStream.Write(buf, 0, buf.Length);
        }
        catch { }
        finally
        {
            try { ctx.Response.OutputStream.Close(); ctx.Response.Close(); } catch { }
        }
    }
}