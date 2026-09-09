using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Reflection;
using System.Text;
using UnityEditor;
using UnityEngine;
using UnityEngine.SceneManagement;

[InitializeOnLoad]
public static class RuneBridge
{
    private static HttpListener listener;
    private const string PORT = "8088";

    private static Type ResolveType(string typeName)
    {
        if (string.IsNullOrEmpty(typeName)) return null;

        Type t = Type.GetType(typeName);
        if (t != null) return t;

        t = Type.GetType($"UnityEngine.{typeName}, UnityEngine") ??
            Type.GetType($"UnityEngine.{typeName}, UnityEngine.CoreModule") ??
            Type.GetType($"UnityEngine.{typeName}, UnityEngine.PhysicsModule");
        if (t != null) return t;

        foreach (var asm in AppDomain.CurrentDomain.GetAssemblies())
        {
            t = asm.GetType(typeName) ?? asm.GetType($"UnityEngine.{typeName}");
            if (t != null) return t;
        }

        return null;
    }

    static RuneBridge()
    {
        try
        {
            listener = new HttpListener();
            listener.Prefixes.Add($"http://localhost:{PORT}/");
            listener.Start();
            listener.BeginGetContext(OnRequest, null);
            Debug.Log($"[RuneBridge] Server listening on http://localhost:{PORT}/");
        }
        catch (Exception ex)
        {
            Debug.LogError($"[RuneBridge] Failed to start HTTP server: {ex.Message}");
        }
    }

    private static void OnRequest(IAsyncResult result)
    {
        if (listener == null || !listener.IsListening) return;

        HttpListenerContext context = listener.EndGetContext(result);
        listener.BeginGetContext(OnRequest, null);

        // Unity API calls must run on the Main Thread
        EditorApplication.delayCall += () => HandleCommand(context);
    }

    private static void HandleCommand(HttpListenerContext context)
    {
        string path = context.Request.Url.AbsolutePath;
        string method = context.Request.HttpMethod;

        try
        {
            if (path == "/scene/inspect" && method == "GET")
            {
                SendResponse(context, GetSceneHierarchyJSON(), HttpStatusCode.OK);
            }
            else if (path == "/scene/validate-references" && method == "GET")
            {
                SendResponse(context, ValidateSceneReferences(), HttpStatusCode.OK);
            }
            else if (path == "/object/inspect-components" && method == "GET")
            {
                string objectName = context.Request.QueryString["objectName"];
                if (string.IsNullOrEmpty(objectName))
                {
                    SendResponse(context, "{\"error\":\"Missing 'objectName' query parameter\"}", HttpStatusCode.BadRequest);
                    return;
                }
                SendResponse(context, InspectGameObjectComponents(objectName), HttpStatusCode.OK);
            }
            else if (path == "/object/assign-reference" && method == "POST")
            {
                string body = ReadRequestBody(context);
                bool success = AssignObjectReference(body);
                string responseJson = success ? "{\"status\":\"success\"}" : "{\"status\":\"error\",\"message\":\"Failed to assign reference\"}";
                SendResponse(context, responseJson, success ? HttpStatusCode.OK : HttpStatusCode.BadRequest);
            }
            else if (path == "/object/set-property" && method == "POST")
            {
                string body = ReadRequestBody(context);
                bool success = SetComponentProperty(body);
                string responseJson = success ? "{\"status\":\"success\"}" : "{\"status\":\"error\",\"message\":\"Failed to set property\"}";
                SendResponse(context, responseJson, success ? HttpStatusCode.OK : HttpStatusCode.BadRequest);
            }

            else if (path == "/object/create" && method == "POST")
            {
                SendResponse(context, CreateGameObject(ReadRequestBody(context)), HttpStatusCode.OK);
            }
            else if (path == "/object/add-component" && method == "POST")
            {
                SendResponse(context, AddComponent(ReadRequestBody(context)), HttpStatusCode.OK);
            }
            else if (path == "/object/destroy" && method == "POST")
            {
                SendResponse(context, DestroyGameObject(ReadRequestBody(context)), HttpStatusCode.OK);
            }
            else if (path == "/prefab/instantiate" && method == "POST")
            {
                SendResponse(context, InstantiatePrefab(ReadRequestBody(context)), HttpStatusCode.OK);
            }
            else if (path == "/asset/find" && method == "GET")
            {
                string filter = context.Request.QueryString["filter"] ?? "";
                SendResponse(context, FindAssets(filter), HttpStatusCode.OK);
            }
            else if (path == "/asset/refresh" && method == "POST")
            {
                AssetDatabase.Refresh();
                SendResponse(context, "{\"status\":\"success\",\"message\":\"AssetDatabase refreshed\"}", HttpStatusCode.OK);
            }
            else if (path == "/console/logs" && method == "GET")
            {
                SendResponse(context, GetConsoleLogs(), HttpStatusCode.OK);
            }
            else if (path == "/editor/play-mode" && method == "POST")
            {
                SendResponse(context, TogglePlayMode(ReadRequestBody(context)), HttpStatusCode.OK);
            }
            else
            {
                SendResponse(context, "{\"error\":\"Endpoint not found\"}", HttpStatusCode.NotFound);
            }
        }
        catch (Exception ex)
        {
            SendResponse(context, $"{{\"error\":\"{EscapeJsonString(ex.Message)}\"}}", HttpStatusCode.InternalServerError);
        }
    }

    private static string ReadRequestBody(HttpListenerContext context)
    {
        using (var reader = new StreamReader(context.Request.InputStream, context.Request.ContentEncoding))
        {
            return reader.ReadToEnd();
        }
    }


    [Serializable]
    private struct CreateObjectPayload
    {
        public string name;
        public string primitiveType;
    }

    private static string CreateGameObject(string jsonPayload)
    {
        CreateObjectPayload payload = JsonUtility.FromJson<CreateObjectPayload>(jsonPayload);
        string name = string.IsNullOrEmpty(payload.name) ? "GameObject" : payload.name;

        GameObject go;
        if (!string.IsNullOrEmpty(payload.primitiveType) && Enum.TryParse(payload.primitiveType, true, out PrimitiveType type))
        {
            go = GameObject.CreatePrimitive(type);
        }
        else
        {
            go = new GameObject(name);
        }

        go.name = name;
        Undo.RegisterCreatedObjectUndo(go, $"Rune Create {name}");
        return $"{{\"status\":\"success\",\"objectName\":\"{EscapeJsonString(go.name)}\"}}";
    }

    [Serializable]
    private struct AddComponentPayload
    {
        public string objectName;
        public string componentType;
    }

    private static string AddComponent(string jsonPayload)
    {
        AddComponentPayload payload = JsonUtility.FromJson<AddComponentPayload>(jsonPayload);
        GameObject go = GameObject.Find(payload.objectName);
        if (go == null) return "{\"status\":\"error\",\"message\":\"GameObject not found\"}";

        Type t = Type.GetType(payload.componentType) ?? Type.GetType($"{payload.componentType}, UnityEngine");
        if (t == null)
        {
            foreach (var asm in AppDomain.CurrentDomain.GetAssemblies())
            {
                t = asm.GetType(payload.componentType);
                if (t != null) break;
            }
        }

        if (t == null) return $"{{\"status\":\"error\",\"message\":\"Component type '{EscapeJsonString(payload.componentType)}' not found\"}}";

        Component comp = Undo.AddComponent(go, t);
        return $"{{\"status\":\"success\",\"component\":\"{EscapeJsonString(comp.GetType().Name)}\",\"attachedTo\":\"{EscapeJsonString(go.name)}\"}}";
    }

    [Serializable]
    private struct DestroyObjectPayload
    {
        public string objectName;
    }

    private static string DestroyGameObject(string jsonPayload)
    {
        DestroyObjectPayload payload = JsonUtility.FromJson<DestroyObjectPayload>(jsonPayload);
        GameObject go = GameObject.Find(payload.objectName);
        if (go == null) return "{\"status\":\"error\",\"message\":\"GameObject not found\"}";

        Undo.DestroyObjectImmediate(go);
        return $"{{\"status\":\"success\",\"destroyed\":\"{EscapeJsonString(payload.objectName)}\"}}";
    }

    [Serializable]
    private struct InstantiatePrefabPayload
    {
        public string prefabPath;
    }

    private static string InstantiatePrefab(string jsonPayload)
    {
        InstantiatePrefabPayload payload = JsonUtility.FromJson<InstantiatePrefabPayload>(jsonPayload);
        GameObject prefab = AssetDatabase.LoadAssetAtPath<GameObject>(payload.prefabPath);

        if (prefab == null) return $"{{\"status\":\"error\",\"message\":\"Prefab not found at '{EscapeJsonString(payload.prefabPath)}'\"}}";

        GameObject instance = (GameObject)PrefabUtility.InstantiatePrefab(prefab);
        Undo.RegisterCreatedObjectUndo(instance, $"Rune Instantiate {instance.name}");
        return $"{{\"status\":\"success\",\"objectName\":\"{EscapeJsonString(instance.name)}\"}}";
    }

    private static string FindAssets(string filter)
    {
        string[] guids = AssetDatabase.FindAssets(filter);
        StringBuilder sb = new StringBuilder();
        sb.Append("{\"assets\":[");

        for (int i = 0; i < guids.Length; i++)
        {
            if (i > 0) sb.Append(",");
            string path = AssetDatabase.GUIDToAssetPath(guids[i]);
            sb.Append($"\"{EscapeJsonString(path)}\"");
        }

        sb.Append("]}");
        return sb.ToString();
    }

    private static string GetConsoleLogs()
    {
        MethodInfo getCountsMethod = typeof(EditorApplication).Assembly
            .GetType("UnityEditor.LogEntries")
            ?.GetMethod("GetCountsByType", BindingFlags.Static | BindingFlags.Public);

        int errorCount = 0, warningCount = 0, logCount = 0;
        if (getCountsMethod != null)
        {
            object[] args = new object[] { 0, 0, 0 };
            getCountsMethod.Invoke(null, args);
            errorCount = (int)args[0];
            warningCount = (int)args[1];
            logCount = (int)args[2];
        }

        return $"{{\"status\":\"success\",\"errors\":{errorCount},\"warnings\":{warningCount},\"logs\":{logCount}}}";
    }

    [Serializable]
    private struct PlayModePayload
    {
        public string state;
    }

    private static string TogglePlayMode(string jsonPayload)
    {
        PlayModePayload payload = JsonUtility.FromJson<PlayModePayload>(jsonPayload);
        string state = payload.state?.ToLower();

        if (state == "play") EditorApplication.isPlaying = true;
        else if (state == "stop") EditorApplication.isPlaying = false;
        else if (state == "pause") EditorApplication.isPaused = !EditorApplication.isPaused;

        return $"{{\"status\":\"success\",\"isPlaying\":{EditorApplication.isPlaying.ToString().ToLower()},\"isPaused\":{EditorApplication.isPaused.ToString().ToLower()}}}";
    }


    private static string GetSceneHierarchyJSON()
    {
        Scene activeScene = SceneManager.GetActiveScene();
        GameObject[] rootObjects = activeScene.GetRootGameObjects();

        StringBuilder sb = new StringBuilder();
        sb.Append("{");
        sb.Append($"\"sceneName\":\"{EscapeJsonString(activeScene.name)}\",");
        sb.Append("\"rootObjects\":[");

        for (int i = 0; i < rootObjects.Length; i++)
        {
            SerializeGameObject(rootObjects[i], sb);
            if (i < rootObjects.Length - 1) sb.Append(",");
        }

        sb.Append("]}");
        return sb.ToString();
    }

    private static void SerializeGameObject(GameObject go, StringBuilder sb)
    {
        sb.Append("{");
        sb.Append($"\"name\":\"{EscapeJsonString(go.name)}\",");
        sb.Append($"\"active\":{(go.activeSelf ? "true" : "false")},");

        sb.Append("\"components\":[");
        Component[] components = go.GetComponents<Component>();
        bool firstComp = true;
        foreach (Component comp in components)
        {
            if (comp == null) continue;
            if (!firstComp) sb.Append(",");
            sb.Append("{");
            sb.Append($"\"type\":\"{EscapeJsonString(comp.GetType().Name)}\"");
            sb.Append("}");
            firstComp = false;
        }
        sb.Append("],");

        sb.Append("\"children\":[");
        int childCount = go.transform.childCount;
        for (int i = 0; i < childCount; i++)
        {
            SerializeGameObject(go.transform.GetChild(i).gameObject, sb);
            if (i < childCount - 1) sb.Append(",");
        }
        sb.Append("]");

        sb.Append("}");
    }


    private static string InspectGameObjectComponents(string objectName)
    {
        GameObject target = GameObject.Find(objectName);
        if (target == null) return "{\"error\":\"GameObject not found\"}";

        StringBuilder sb = new StringBuilder();
        sb.Append("{");
        sb.Append($"\"objectName\":\"{EscapeJsonString(target.name)}\",");
        sb.Append("\"components\":[");

        Component[] components = target.GetComponents<Component>();
        for (int i = 0; i < components.Length; i++)
        {
            Component comp = components[i];
            if (comp == null)
            {
                if (i > 0) sb.Append(",");
                sb.Append("{\"componentType\":\"MissingComponent\",\"hasMissingScript\":true}");
                continue;
            }

            if (i > 0) sb.Append(",");
            sb.Append("{");
            sb.Append($"\"componentType\":\"{EscapeJsonString(comp.GetType().Name)}\",");
            sb.Append($"\"fullType\":\"{EscapeJsonString(comp.GetType().FullName)}\",");
            sb.Append("\"fields\":[");

            SerializedObject so = new SerializedObject(comp);
            SerializedProperty prop = so.GetIterator();

            bool firstProp = true;
            bool enterChildren = true;

            while (prop.NextVisible(enterChildren))
            {
                enterChildren = false;

                if (prop.name == "m_Script") continue;

                if (!firstProp) sb.Append(",");
                firstProp = false;

                sb.Append("{");
                sb.Append($"\"field\":\"{EscapeJsonString(prop.name)}\",");
                sb.Append($"\"type\":\"{EscapeJsonString(prop.propertyType.ToString())}\",");

                if (prop.propertyType == SerializedPropertyType.ObjectReference)
                {
                    bool isNull = prop.objectReferenceValue == null;
                    sb.Append($"\"isAssigned\":{(!isNull).ToString().ToLower()},");

                    if (!isNull)
                    {
                        sb.Append($"\"assignedName\":\"{EscapeJsonString(prop.objectReferenceValue.name)}\",");
                        sb.Append($"\"assignedType\":\"{EscapeJsonString(prop.objectReferenceValue.GetType().Name)}\"");
                    }
                    else
                    {
                        sb.Append("\"assignedName\":null,\"assignedType\":null");
                    }
                }
                else
                {
                    sb.Append("\"isAssigned\":true,");
                    sb.Append($"\"value\":\"{EscapeJsonString(GetPropValueAsString(prop))}\"");
                }

                sb.Append("}");
            }

            sb.Append("]}");
        }

        sb.Append("]}");
        return sb.ToString();
    }

    private static string ValidateSceneReferences()
    {
        Scene activeScene = SceneManager.GetActiveScene();
        GameObject[] rootObjects = activeScene.GetRootGameObjects();

        StringBuilder sb = new StringBuilder();
        sb.Append("{");
        sb.Append($"\"sceneName\":\"{EscapeJsonString(activeScene.name)}\",");
        sb.Append("\"unassignedReferences\":[");

        bool firstUnassigned = true;

        List<GameObject> allObjects = new List<GameObject>();
        foreach (GameObject root in rootObjects)
        {
            GetGameObjectsRecursive(root, allObjects);
        }

        foreach (GameObject go in allObjects)
        {
            Component[] components = go.GetComponents<Component>();
            foreach (Component comp in components)
            {
                if (comp == null) continue;

                SerializedObject so = new SerializedObject(comp);
                SerializedProperty prop = so.GetIterator();
                bool enterChildren = true;

                while (prop.NextVisible(enterChildren))
                {
                    enterChildren = false;

                    if (prop.name == "m_Script") continue;

                    if (prop.propertyType == SerializedPropertyType.ObjectReference && prop.objectReferenceValue == null)
                    {
                        if (!firstUnassigned) sb.Append(",");
                        firstUnassigned = false;

                        sb.Append("{");
                        sb.Append($"\"gameObject\":\"{EscapeJsonString(go.name)}\",");
                        sb.Append($"\"component\":\"{EscapeJsonString(comp.GetType().Name)}\",");
                        sb.Append($"\"field\":\"{EscapeJsonString(prop.name)}\"");
                        sb.Append("}");
                    }
                }
            }
        }

        sb.Append("]}");
        return sb.ToString();
    }

    private static void GetGameObjectsRecursive(GameObject go, List<GameObject> list)
    {
        list.Add(go);
        foreach (Transform child in go.transform)
        {
            GetGameObjectsRecursive(child.gameObject, list);
        }
    }


    [Serializable]
    private struct ReferencePayload
    {
        public string objectName;
        public string componentType;
        public string fieldName;
        public string targetObjectName;
        public string targetComponentType;
    }

    private static bool AssignObjectReference(string jsonPayload)
    {
        ReferencePayload payload = JsonUtility.FromJson<ReferencePayload>(jsonPayload);

        GameObject sourceObj = GameObject.Find(payload.objectName);
        GameObject targetObj = GameObject.Find(payload.targetObjectName);

        if (sourceObj == null || targetObj == null)
        {
            Debug.LogWarning($"[RuneBridge] Assign reference failed: GameObject '{payload.objectName}' or '{payload.targetObjectName}' not found.");
            return false;
        }

        Type compType = ResolveType(payload.componentType);
        Component comp = compType != null ? sourceObj.GetComponent(compType) : sourceObj.GetComponent(payload.componentType);

        if (comp == null)
        {
            Debug.LogWarning($"[RuneBridge] Component '{payload.componentType}' not found on '{payload.objectName}'");
            return false;
        }

        SerializedObject so = new SerializedObject(comp);
        SerializedProperty prop = so.FindProperty(payload.fieldName);

        if (prop != null && prop.propertyType == SerializedPropertyType.ObjectReference)
        {
            Undo.RecordObject(comp, "Rune Assign Reference");

            if (!string.IsNullOrEmpty(payload.targetComponentType))
            {
                Type targetCompType = ResolveType(payload.targetComponentType);
                Component targetComp = targetCompType != null ? targetObj.GetComponent(targetCompType) : targetObj.GetComponent(payload.targetComponentType);
                prop.objectReferenceValue = targetComp;
            }
            else
            {
                Type fieldType = comp.GetType().GetField(payload.fieldName, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance)?.FieldType;

                if (fieldType != null && typeof(Component).IsAssignableFrom(fieldType))
                {
                    prop.objectReferenceValue = targetObj.GetComponent(fieldType);
                }
                else
                {
                    prop.objectReferenceValue = targetObj;
                }
            }

            so.ApplyModifiedProperties();
            return true;
        }

        Debug.LogWarning($"[RuneBridge] Field '{payload.fieldName}' is not a valid ObjectReference on Component '{payload.componentType}'");
        return false;
    }

    [Serializable]
    private struct PropertyPayload
    {
        public string objectName;
        public string componentType;
        public string propertyName;
        public string value;
    }

    private static bool SetComponentProperty(string jsonPayload)
    {
        PropertyPayload payload = JsonUtility.FromJson<PropertyPayload>(jsonPayload);

        GameObject target = GameObject.Find(payload.objectName);
        if (target == null)
        {
            Debug.LogWarning($"[RuneBridge] Could not find GameObject: {payload.objectName}");
            return false;
        }

        if (string.Equals(payload.componentType, "GameObject", StringComparison.OrdinalIgnoreCase))
        {
            Undo.RecordObject(target, "Rune Modified GameObject");

            if (payload.propertyName.Equals("active", StringComparison.OrdinalIgnoreCase) ||
                payload.propertyName.Equals("m_IsActive", StringComparison.OrdinalIgnoreCase))
            {
                if (bool.TryParse(payload.value, out bool activeVal))
                {
                    target.SetActive(activeVal);
                    return true;
                }
            }
            else if (payload.propertyName.Equals("name", StringComparison.OrdinalIgnoreCase) ||
                     payload.propertyName.Equals("m_Name", StringComparison.OrdinalIgnoreCase))
            {
                target.name = payload.value;
                return true;
            }
            else if (payload.propertyName.Equals("tag", StringComparison.OrdinalIgnoreCase))
            {
                target.tag = payload.value;
                return true;
            }
            else if (payload.propertyName.Equals("layer", StringComparison.OrdinalIgnoreCase))
            {
                if (int.TryParse(payload.value, out int layerVal))
                {
                    target.layer = layerVal;
                    return true;
                }
            }

            SerializedObject goSo = new SerializedObject(target);
            SerializedProperty goProp = goSo.FindProperty(payload.propertyName);
            if (goProp != null)
            {
                ApplyPropertyValue(goProp, payload.value);
                goSo.ApplyModifiedProperties();
                return true;
            }

            return false;
        }

        Component comp = target.GetComponent(payload.componentType);
        if (comp == null && payload.componentType.Equals("Transform", StringComparison.OrdinalIgnoreCase))
        {
            comp = target.transform;
        }

        if (comp == null)
        {
            Debug.LogWarning($"[RuneBridge] Could not find Component '{payload.componentType}' on '{payload.objectName}'");
            return false;
        }

        Undo.RecordObject(comp, "Rune Modified Property");

        SerializedObject so = new SerializedObject(comp);
        SerializedProperty prop = so.FindProperty(payload.propertyName);

        if (prop == null)
        {
            Debug.LogWarning($"[RuneBridge] Property '{payload.propertyName}' not found on Component '{payload.componentType}'");
            return false;
        }

        bool success = ApplyPropertyValue(prop, payload.value);
        if (success)
        {
            so.ApplyModifiedProperties();
        }
        return success;
    }

    private static bool ApplyPropertyValue(SerializedProperty prop, string rawValue)
    {
        switch (prop.propertyType)
        {
            case SerializedPropertyType.Float:
                if (float.TryParse(rawValue, out float fVal)) { prop.floatValue = fVal; return true; }
                break;
            case SerializedPropertyType.Integer:
                if (int.TryParse(rawValue, out int iVal)) { prop.intValue = iVal; return true; }
                break;
            case SerializedPropertyType.Boolean:
                if (bool.TryParse(rawValue, out bool bVal)) { prop.boolValue = bVal; return true; }
                break;
            case SerializedPropertyType.String:
                prop.stringValue = rawValue;
                return true;
            case SerializedPropertyType.Enum:
                if (int.TryParse(rawValue, out int enumIdx))
                {
                    prop.enumValueIndex = enumIdx;
                    return true;
                }
                break;
        }
        return false;
    }

    private static string GetPropValueAsString(SerializedProperty prop)
    {
        return prop.propertyType switch
        {
            SerializedPropertyType.Integer => prop.intValue.ToString(),
            SerializedPropertyType.Boolean => prop.boolValue.ToString(),
            SerializedPropertyType.Float => prop.floatValue.ToString(),
            SerializedPropertyType.String => prop.stringValue,
            SerializedPropertyType.Enum => prop.enumNames[prop.enumValueIndex],
            _ => prop.propertyType.ToString()
        };
    }

    private static string EscapeJsonString(string str)
    {
        if (string.IsNullOrEmpty(str)) return "";
        return str.Replace("\\", "\\\\").Replace("\"", "\\\"").Replace("\n", "\\n").Replace("\r", "\\r").Replace("\t", "\\t");
    }

    private static void SendResponse(HttpListenerContext context, string json, HttpStatusCode status)
    {
        try
        {
            if (context?.Response == null) return;

            byte[] buffer = Encoding.UTF8.GetBytes(json ?? "{}");

            context.Response.StatusCode = (int)status;
            context.Response.ContentType = "application/json";
            context.Response.ContentEncoding = Encoding.UTF8;
            context.Response.ContentLength64 = buffer.Length;

            context.Response.OutputStream.Write(buffer, 0, buffer.Length);
        }
        catch (ObjectDisposedException) { }
        catch (InvalidOperationException ex)
        {
            Debug.LogWarning($"[RuneBridge] Response headers already committed: {ex.Message}");
        }
        catch (Exception ex)
        {
            Debug.LogError($"[RuneBridge] Response write error: {ex.Message}");
        }
        finally
        {
            try
            {
                context?.Response?.OutputStream?.Close();
                context?.Response?.Close();
            }
            catch { }
        }
    }

}

