import { useState, useCallback } from "react";
import { SegmentedControl } from "@/features/usage/ChartControls";
import { useGroups } from "./useGroups";
import type { GroupWithMembers, BudgetBasis } from "./types";

// ── Validation helpers (mirror backend rules) ─────────────────────────────────

function validateName(
  name: string,
  existingNames: string[],
): string | null {
  const trimmed = name.trim();
  if (!trimmed) return "El nombre no puede estar vacío.";
  const collision = existingNames.some(
    (n) => n.toLowerCase() === trimmed.toLowerCase(),
  );
  if (collision) return `Ya existe un grupo con ese nombre.`;
  return null;
}

function validateBudgetValue(
  basis: BudgetBasis | null,
  rawValue: string,
): string | null {
  if (basis === null) return null;
  const v = parseFloat(rawValue);
  if (isNaN(v) || v <= 0) return "El valor debe ser mayor que 0.";
  if (basis === "share" && v > 100) return "El porcentaje no puede superar 100.";
  return null;
}

// ── Basis toggle options ───────────────────────────────────────────────────────

const BASIS_OPTIONS: { value: BudgetBasis | "none"; label: string }[] = [
  { value: "none", label: "Sin tope" },
  { value: "share", label: "% costo" },
  { value: "usd", label: "$ USD" },
];

// ── CreateGroupForm ───────────────────────────────────────────────────────────

interface CreateGroupFormProps {
  existingNames: string[];
  onCreate: (
    name: string,
    budgetBasis: BudgetBasis | null,
    budgetValue: number | null,
  ) => Promise<void>;
}

function CreateGroupForm({ existingNames, onCreate }: CreateGroupFormProps) {
  const [name, setName] = useState("");
  const [basisMode, setBasisMode] = useState<BudgetBasis | "none">("none");
  const [valueStr, setValueStr] = useState("");
  const [nameError, setNameError] = useState<string | null>(null);
  const [valueError, setValueError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);

  const basis: BudgetBasis | null = basisMode === "none" ? null : basisMode;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const nErr = validateName(name, existingNames);
    const vErr = validateBudgetValue(basis, valueStr);
    setNameError(nErr);
    setValueError(vErr);
    if (nErr || vErr) return;

    setSubmitting(true);
    setServerError(null);
    try {
      const budgetValue = basis !== null ? parseFloat(valueStr) : null;
      await onCreate(name.trim(), basis, budgetValue);
      setName("");
      setBasisMode("none");
      setValueStr("");
    } catch (err) {
      setServerError(err instanceof Error ? err.message : "Error al crear el grupo.");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form
      onSubmit={(e) => void handleSubmit(e)}
      style={{ display: "flex", flexDirection: "column", gap: "var(--space-xs)" }}
    >
      <h3
        style={{
          margin: 0,
          fontSize: 11,
          fontWeight: 700,
          textTransform: "uppercase",
          letterSpacing: "0.06em",
          color: "var(--text-muted)",
        }}
      >
        Nuevo grupo
      </h3>

      {/* Name */}
      <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
        <input
          type="text"
          placeholder="Nombre del grupo"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setNameError(null);
          }}
          aria-label="Nombre del grupo"
          style={{ width: "100%" }}
        />
        {nameError && (
          <span style={{ fontSize: 11, color: "var(--danger)" }}>{nameError}</span>
        )}
      </div>

      {/* Budget basis toggle */}
      <SegmentedControl
        label="Tope"
        options={BASIS_OPTIONS}
        value={basisMode}
        onChange={(v) => {
          setBasisMode(v);
          setValueStr("");
          setValueError(null);
        }}
      />

      {/* Budget value input */}
      {basis !== null && (
        <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
            {basis === "usd" && (
              <span style={{ fontSize: 12, color: "var(--text-muted)" }}>$</span>
            )}
            <input
              type="number"
              placeholder={basis === "share" ? "0–100" : "0.00"}
              value={valueStr}
              min={0.01}
              max={basis === "share" ? 100 : undefined}
              step="any"
              onChange={(e) => {
                setValueStr(e.target.value);
                setValueError(null);
              }}
              aria-label={basis === "share" ? "Porcentaje máximo" : "Monto máximo en USD"}
              style={{ width: 80 }}
            />
            {basis === "share" && (
              <span style={{ fontSize: 12, color: "var(--text-muted)" }}>%</span>
            )}
          </div>
          {valueError && (
            <span style={{ fontSize: 11, color: "var(--danger)" }}>{valueError}</span>
          )}
        </div>
      )}

      {serverError && (
        <span style={{ fontSize: 11, color: "var(--danger)" }}>{serverError}</span>
      )}

      <button
        type="submit"
        disabled={submitting}
        style={{
          alignSelf: "flex-start",
          fontSize: 12,
          fontWeight: 500,
          padding: "4px 12px",
          background: "var(--accent)",
          color: "var(--accent-text)",
          border: "none",
          borderRadius: "var(--radius-md)",
          cursor: submitting ? "not-allowed" : "pointer",
          opacity: submitting ? 0.6 : 1,
        }}
      >
        {submitting ? "Creando…" : "Crear grupo"}
      </button>
    </form>
  );
}

// ── GroupRow (editable) ───────────────────────────────────────────────────────

interface GroupRowProps {
  gwm: GroupWithMembers;
  allGroupNames: string[];
  projectNames: string[];
  allGroups: GroupWithMembers[];
  onUpdate: (
    id: number,
    name: string,
    budgetBasis: BudgetBasis | null,
    budgetValue: number | null,
  ) => Promise<void>;
  onDelete: (id: number) => Promise<void>;
  onAssign: (projectName: string, groupId: number) => Promise<void>;
  onUnassign: (projectName: string) => Promise<void>;
}

function GroupRow({
  gwm,
  allGroupNames,
  projectNames,
  allGroups,
  onUpdate,
  onDelete,
  onAssign,
  onUnassign,
}: GroupRowProps) {
  const { group, members } = gwm;

  const [editing, setEditing] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  // Edit form state
  const [editName, setEditName] = useState(group.name);
  const [editBasisMode, setEditBasisMode] = useState<BudgetBasis | "none">(
    group.budgetBasis ?? "none",
  );
  const [editValueStr, setEditValueStr] = useState(
    group.budgetValue !== null ? String(group.budgetValue) : "",
  );
  const [nameError, setNameError] = useState<string | null>(null);
  const [valueError, setValueError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);

  const editBasis: BudgetBasis | null = editBasisMode === "none" ? null : editBasisMode;

  // Names of other groups (to check uniqueness)
  const otherNames = allGroupNames.filter((n) => n !== group.name);

  const handleSave = async () => {
    const nErr = validateName(editName, otherNames);
    const vErr = validateBudgetValue(editBasis, editValueStr);
    setNameError(nErr);
    setValueError(vErr);
    if (nErr || vErr) return;

    setSaving(true);
    setServerError(null);
    try {
      const budgetValue = editBasis !== null ? parseFloat(editValueStr) : null;
      await onUpdate(group.id, editName.trim(), editBasis, budgetValue);
      setEditing(false);
    } catch (err) {
      setServerError(err instanceof Error ? err.message : "Error al guardar.");
    } finally {
      setSaving(false);
    }
  };

  const handleCancelEdit = () => {
    setEditing(false);
    setEditName(group.name);
    setEditBasisMode(group.budgetBasis ?? "none");
    setEditValueStr(group.budgetValue !== null ? String(group.budgetValue) : "");
    setNameError(null);
    setValueError(null);
    setServerError(null);
  };

  const handleDelete = async () => {
    await onDelete(group.id);
  };

  // Cap display label
  const capLabel =
    group.budgetBasis === "share"
      ? `${group.budgetValue}% costo local`
      : group.budgetBasis === "usd"
        ? `$${group.budgetValue?.toFixed(2)}`
        : "Sin tope";

  // Find which group a project belongs to (for chip display)
  const getProjectGroup = (projectName: string): string | null => {
    for (const g of allGroups) {
      if (g.members.includes(projectName)) return g.group.name;
    }
    return null;
  };

  return (
    <div
      style={{
        border: "1px solid var(--border)",
        borderRadius: "var(--radius-md)",
        overflow: "hidden",
      }}
    >
      {/* Group header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "var(--space-xs) var(--space-sm)",
          background: "var(--surface-2)",
          borderBottom: editing ? "1px solid var(--border)" : undefined,
        }}
      >
        {editing ? (
          <span
            style={{ fontSize: 13, fontWeight: 600, color: "var(--text)" }}
          >
            Editando: {group.name}
          </span>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            <span style={{ fontSize: 13, fontWeight: 600, color: "var(--text)" }}>
              {group.name}
            </span>
            <span style={{ fontSize: 11, color: "var(--text-muted)" }}>{capLabel}</span>
          </div>
        )}

        <div style={{ display: "flex", gap: 4 }}>
          {!editing && (
            <button
              onClick={() => setEditing(true)}
              style={{ fontSize: 11, padding: "2px 8px" }}
            >
              Editar
            </button>
          )}
          {!editing && !confirmDelete && (
            <button
              onClick={() => setConfirmDelete(true)}
              style={{
                fontSize: 11,
                padding: "2px 8px",
                color: "var(--danger)",
                borderColor: "var(--danger)",
              }}
            >
              Borrar
            </button>
          )}
          {confirmDelete && (
            <span style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 11 }}>
              <span style={{ color: "var(--text-muted)" }}>
                Borrar &laquo;{group.name}&raquo;? Sus proyectos vuelven a otros.
              </span>
              <button
                onClick={() => void handleDelete()}
                style={{
                  fontSize: 11,
                  padding: "2px 8px",
                  background: "var(--danger)",
                  color: "white",
                  border: "none",
                  borderRadius: "var(--radius-sm)",
                }}
              >
                Sí, borrar
              </button>
              <button
                onClick={() => setConfirmDelete(false)}
                style={{ fontSize: 11, padding: "2px 8px" }}
              >
                Cancelar
              </button>
            </span>
          )}
        </div>
      </div>

      {/* Edit form */}
      {editing && (
        <div
          style={{
            padding: "var(--space-sm)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--space-xs)",
            borderBottom: "1px solid var(--border)",
          }}
        >
          {/* Name */}
          <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
            <input
              type="text"
              value={editName}
              onChange={(e) => {
                setEditName(e.target.value);
                setNameError(null);
              }}
              aria-label="Nombre del grupo"
            />
            {nameError && (
              <span style={{ fontSize: 11, color: "var(--danger)" }}>{nameError}</span>
            )}
          </div>

          {/* Budget basis */}
          <SegmentedControl
            label="Tope"
            options={BASIS_OPTIONS}
            value={editBasisMode}
            onChange={(v) => {
              setEditBasisMode(v);
              setEditValueStr("");
              setValueError(null);
            }}
          />

          {/* Budget value */}
          {editBasis !== null && (
            <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                {editBasis === "usd" && (
                  <span style={{ fontSize: 12, color: "var(--text-muted)" }}>$</span>
                )}
                <input
                  type="number"
                  placeholder={editBasis === "share" ? "0–100" : "0.00"}
                  value={editValueStr}
                  min={0.01}
                  max={editBasis === "share" ? 100 : undefined}
                  step="any"
                  onChange={(e) => {
                    setEditValueStr(e.target.value);
                    setValueError(null);
                  }}
                  aria-label={
                    editBasis === "share" ? "Porcentaje máximo" : "Monto máximo en USD"
                  }
                  style={{ width: 80 }}
                />
                {editBasis === "share" && (
                  <span style={{ fontSize: 12, color: "var(--text-muted)" }}>%</span>
                )}
              </div>
              {valueError && (
                <span style={{ fontSize: 11, color: "var(--danger)" }}>{valueError}</span>
              )}
            </div>
          )}

          {serverError && (
            <span style={{ fontSize: 11, color: "var(--danger)" }}>{serverError}</span>
          )}

          <div style={{ display: "flex", gap: 4 }}>
            <button
              onClick={() => void handleSave()}
              disabled={saving}
              style={{
                fontSize: 12,
                fontWeight: 500,
                padding: "4px 12px",
                background: "var(--accent)",
                color: "var(--accent-text)",
                border: "none",
                borderRadius: "var(--radius-sm)",
                cursor: saving ? "not-allowed" : "pointer",
                opacity: saving ? 0.6 : 1,
              }}
            >
              {saving ? "Guardando…" : "Guardar"}
            </button>
            <button
              onClick={handleCancelEdit}
              disabled={saving}
              style={{ fontSize: 12, padding: "4px 12px" }}
            >
              Cancelar
            </button>
          </div>
        </div>
      )}

      {/* Members / project assignment */}
      <div style={{ padding: "var(--space-xs) var(--space-sm)" }}>
        <span
          style={{
            fontSize: 10,
            fontWeight: 700,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: "var(--text-subtle)",
          }}
        >
          Proyectos asignados ({members.length})
        </span>
        <div
          style={{
            marginTop: 4,
            display: "flex",
            flexWrap: "wrap",
            gap: 4,
          }}
        >
          {members.map((proj) => (
            <span
              key={proj}
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                fontSize: 11,
                padding: "2px 6px",
                background: "var(--accent-soft)",
                color: "var(--accent)",
                borderRadius: "var(--radius-full)",
                border: "1px solid var(--accent-glow)",
              }}
            >
              {proj}
              <button
                onClick={() => void onUnassign(proj)}
                aria-label={`Desasignar ${proj}`}
                style={{
                  background: "none",
                  border: "none",
                  padding: 0,
                  fontSize: 11,
                  cursor: "pointer",
                  color: "var(--accent)",
                  lineHeight: 1,
                }}
              >
                ×
              </button>
            </span>
          ))}
          {members.length === 0 && (
            <span style={{ fontSize: 11, color: "var(--text-subtle)", fontStyle: "italic" }}>
              Sin proyectos asignados
            </span>
          )}
        </div>
      </div>

      {/* Unassigned projects available to assign */}
      <UnassignedProjectsForGroup
        groupId={group.id}
        projectNames={projectNames}
        members={members}
        getProjectGroup={getProjectGroup}
        onAssign={onAssign}
      />
    </div>
  );
}

// ── UnassignedProjectsForGroup ────────────────────────────────────────────────

interface UnassignedProjectsForGroupProps {
  groupId: number;
  projectNames: string[];
  members: string[];
  getProjectGroup: (projectName: string) => string | null;
  onAssign: (projectName: string, groupId: number) => Promise<void>;
}

function UnassignedProjectsForGroup({
  groupId,
  projectNames,
  members,
  getProjectGroup,
  onAssign,
}: UnassignedProjectsForGroupProps) {
  // Projects that are not in THIS group (available to assign)
  const assignable = projectNames.filter((p) => !members.includes(p));

  if (assignable.length === 0) return null;

  return (
    <div
      style={{
        padding: "var(--space-xs) var(--space-sm)",
        borderTop: "1px solid var(--hairline)",
      }}
    >
      <span
        style={{
          fontSize: 10,
          fontWeight: 700,
          textTransform: "uppercase",
          letterSpacing: "0.06em",
          color: "var(--text-subtle)",
        }}
      >
        Asignar proyectos
      </span>
      <div
        style={{
          marginTop: 4,
          display: "flex",
          flexWrap: "wrap",
          gap: 4,
        }}
      >
        {assignable.map((proj) => {
          const currentGroup = getProjectGroup(proj);
          return (
            <button
              key={proj}
              onClick={() => void onAssign(proj, groupId)}
              title={
                currentGroup
                  ? `Reasignar desde «${currentGroup}»`
                  : "Asignar a este grupo"
              }
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                fontSize: 11,
                padding: "2px 6px",
                background: "var(--surface-3)",
                color: currentGroup ? "var(--text-muted)" : "var(--text)",
                borderRadius: "var(--radius-full)",
                border: "1px solid var(--border)",
                cursor: "pointer",
              }}
            >
              {proj}
              {currentGroup && (
                <span
                  style={{
                    fontSize: 10,
                    background: "var(--surface-2)",
                    color: "var(--text-subtle)",
                    padding: "0 4px",
                    borderRadius: "var(--radius-full)",
                  }}
                >
                  {currentGroup}
                </span>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}

// ── ProjectsOverview (all projects with their group or "sin grupo") ────────────

interface ProjectsOverviewProps {
  projectNames: string[];
  groups: GroupWithMembers[];
  onUnassign: (projectName: string) => Promise<void>;
}

function ProjectsOverview({ projectNames, groups, onUnassign }: ProjectsOverviewProps) {
  const getGroupName = (proj: string): string | null => {
    for (const g of groups) {
      if (g.members.includes(proj)) return g.group.name;
    }
    return null;
  };

  if (projectNames.length === 0) return null;

  return (
    <div>
      <h3
        style={{
          margin: "0 0 var(--space-xs)",
          fontSize: 11,
          fontWeight: 700,
          textTransform: "uppercase",
          letterSpacing: "0.06em",
          color: "var(--text-muted)",
        }}
      >
        Todos los proyectos
      </h3>
      <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
        {projectNames.map((proj) => {
          const groupName = getGroupName(proj);
          return (
            <div
              key={proj}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                padding: "3px 0",
                borderBottom: "1px solid var(--hairline)",
                fontSize: 12,
              }}
            >
              <span
                style={{
                  color: "var(--text)",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                  maxWidth: "55%",
                }}
                title={proj}
              >
                {proj}
              </span>
              <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
                {groupName ? (
                  <>
                    <span
                      style={{
                        fontSize: 11,
                        padding: "1px 6px",
                        background: "var(--accent-soft)",
                        color: "var(--accent)",
                        borderRadius: "var(--radius-full)",
                        border: "1px solid var(--accent-glow)",
                      }}
                    >
                      {groupName}
                    </span>
                    <button
                      onClick={() => void onUnassign(proj)}
                      aria-label={`Quitar ${proj} del grupo`}
                      style={{
                        fontSize: 11,
                        padding: "1px 6px",
                        color: "var(--text-muted)",
                      }}
                    >
                      Quitar
                    </button>
                  </>
                ) : (
                  <span
                    style={{
                      fontSize: 11,
                      color: "var(--text-subtle)",
                      fontStyle: "italic",
                    }}
                  >
                    sin grupo
                  </span>
                )}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ── GroupsEditor (main panel) ─────────────────────────────────────────────────

/**
 * Dashboard panel for managing project groups.
 * Lists groups with their cap and members; allows create/rename/delete/cap/assign.
 * Uses SegmentedControl for basis toggle and native input/select from global.css.
 * No modal, no tab — just a .panel in the dashboard per D16.
 */
export function GroupsEditor() {
  const {
    groups,
    projectNames,
    loading,
    error,
    createGroup,
    updateGroup,
    deleteGroup,
    assignProject,
    unassignProject,
  } = useGroups();

  const handleCreate = useCallback(
    async (
      name: string,
      budgetBasis: BudgetBasis | null,
      budgetValue: number | null,
    ) => {
      await createGroup(name, budgetBasis, budgetValue);
    },
    [createGroup],
  );

  const allGroupNames = groups.map((g) => g.group.name);

  return (
    <div className="panel">
      <div className="panel__header">
        <h2 className="panel__title">Grupos de proyecto</h2>
      </div>
      <div className="panel__body" style={{ display: "flex", flexDirection: "column", gap: "var(--space-lg)" }}>
        {error && (
          <p
            role="alert"
            style={{
              margin: 0,
              fontSize: 12,
              color: "var(--danger)",
              padding: "var(--space-xs) var(--space-sm)",
              background: "color-mix(in srgb, var(--danger) 10%, transparent)",
              border: "1px solid color-mix(in srgb, var(--danger) 30%, transparent)",
              borderRadius: "var(--radius-md)",
            }}
          >
            {error}
          </p>
        )}

        {/* Create form */}
        <CreateGroupForm existingNames={allGroupNames} onCreate={handleCreate} />

        {/* Group list */}
        {loading && groups.length === 0 ? (
          <p style={{ margin: 0, fontSize: 12, color: "var(--text-muted)" }}>
            Cargando grupos…
          </p>
        ) : groups.length === 0 ? (
          <p
            style={{
              margin: 0,
              fontSize: 12,
              color: "var(--text-muted)",
              fontStyle: "italic",
            }}
          >
            No hay grupos definidos. Crea uno arriba.
          </p>
        ) : (
          <>
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-sm)" }}>
              {groups.map((gwm) => (
                <GroupRow
                  key={gwm.group.id}
                  gwm={gwm}
                  allGroupNames={allGroupNames}
                  projectNames={projectNames}
                  allGroups={groups}
                  onUpdate={updateGroup}
                  onDelete={deleteGroup}
                  onAssign={assignProject}
                  onUnassign={unassignProject}
                />
              ))}
            </div>

            {/* Projects overview */}
            {projectNames.length > 0 && (
              <ProjectsOverview
                projectNames={projectNames}
                groups={groups}
                onUnassign={unassignProject}
              />
            )}
          </>
        )}
      </div>
    </div>
  );
}
