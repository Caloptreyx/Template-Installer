CREATE TABLE dev_caloptreyx_templates_templates (
    uuid uuid NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT true,
    egg_uuids uuid[] NOT NULL DEFAULT '{}',
    variables JSONB NOT NULL DEFAULT '[]',
    steps JSONB NOT NULL DEFAULT '[]',
    created TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX dev_caloptreyx_templates_templates_egg_uuids_idx
    ON dev_caloptreyx_templates_templates USING GIN (egg_uuids);

CREATE TABLE dev_caloptreyx_templates_installs (
    uuid uuid NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    server_uuid uuid NOT NULL REFERENCES servers(uuid) ON DELETE CASCADE,
    template_uuid uuid REFERENCES dev_caloptreyx_templates_templates(uuid) ON DELETE SET NULL,
    template_name VARCHAR(255) NOT NULL,
    user_uuid uuid REFERENCES users(uuid) ON DELETE SET NULL,
    status VARCHAR(15) NOT NULL,
    steps JSONB NOT NULL DEFAULT '[]',
    error TEXT,
    created TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished TIMESTAMPTZ
);

CREATE INDEX dev_caloptreyx_templates_installs_server_created_idx
    ON dev_caloptreyx_templates_installs (server_uuid, created DESC);
-- at most one running install per server
CREATE UNIQUE INDEX dev_caloptreyx_templates_installs_running_idx
    ON dev_caloptreyx_templates_installs (server_uuid)
    WHERE status = 'running';
