"""Deep-merge layered JSON configs (defaults, environment, overrides) and validate them."""

import json

DEFAULTS = """
{
  "server": {"host": "127.0.0.1", "port": 8000, "workers": 2},
  "logging": {"level": "INFO", "handlers": ["console"]},
  "features": {"beta": false, "cache": true},
  "name": "app"
}
"""

PRODUCTION = """
{
  "server": {"host": "0.0.0.0", "workers": 8},
  "logging": {"level": "WARNING", "handlers": ["console", "file"]},
  "features": {"cache": true, "metrics": true}
}
"""

OVERRIDES = '{"server": {"port": 9090}, "features": {"beta": null}, "debug": true}'


def deep_merge(base, extra, path=""):
    result = dict(base)
    for key, value in extra.items():
        where = f"{path}.{key}" if path else key
        if value is None:
            if key in result:
                del result[key]
                print(f"  removed {where}")
            continue
        if key in result and isinstance(result[key], dict) and isinstance(value, dict):
            result[key] = deep_merge(result[key], value, where)
        else:
            if key in result and result[key] != value:
                print(f"  {where}: {json.dumps(result[key])} -> {json.dumps(value)}")
            elif key not in result:
                print(f"  added {where}")
            result[key] = value
    return result


def flatten(cfg, prefix=""):
    flat = {}
    for key, value in cfg.items():
        name = prefix + key
        if isinstance(value, dict):
            flat.update(flatten(value, name + "."))
        else:
            flat[name] = value
    return flat


def validate(cfg):
    errors = []
    port = cfg.get("server", {}).get("port", 0)
    if not 1 <= port <= 65535:
        errors.append(f"bad port {port}")
    workers = cfg["server"].get("workers", 1)
    if workers > 4 and cfg["logging"]["level"] == "DEBUG":
        errors.append("too noisy")
    if "file" in cfg["logging"]["handlers"] and "log_path" not in cfg["logging"]:
        errors.append("file handler requires logging.log_path")
    return errors


def main():
    layers = [("defaults", DEFAULTS), ("production", PRODUCTION), ("overrides", OVERRIDES)]
    config = {}
    for name, text in layers:
        print(f"applying {name}:")
        config = deep_merge(config, json.loads(text))
    print(json.dumps(config, indent=2, sort_keys=True))
    for key, value in sorted(flatten(config).items()):
        print(f"{key:<24} = {value!r}")
    errors = validate(config)
    if errors:
        for err in errors:
            print("error:", err)
    else:
        print("config ok")
    assert config["server"]["port"] == 9090, "override lost"
    compact = json.dumps(config["features"], separators=(",", ":"), sort_keys=True)
    print(compact)


if __name__ == "__main__":
    main()
