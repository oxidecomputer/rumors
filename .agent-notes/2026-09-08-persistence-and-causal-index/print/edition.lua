-- Keep the Markdown as the only manuscript; adapt navigation for one PDF.
local chapters = {
  ["README.md"] = "overview",
  ["01-model.md"] = "model",
  ["02-storage.md"] = "storage",
  ["03-operations.md"] = "operations",
  ["04-implementation.md"] = "implementation",
  ["05-review.md"] = "review",
}
local chapter_ids = {"overview", "model", "storage", "operations", "implementation", "review"}

function Pandoc(doc)
  local chapter = 0
  local skip_print_instructions = false
  local blocks = pandoc.List()
  for _, block in ipairs(doc.blocks) do
    if block.t == "Header" and block.level == 1 then
      chapter = chapter + 1
      if chapter == 2 then blocks:insert(pandoc.RawBlock("typst", "]")) end
      block.identifier = chapter_ids[chapter]
      skip_print_instructions = false
      if chapter == 1 then block.content = pandoc.Inlines("Overview") end
      blocks:insert(pandoc.RawBlock("typst", "#pagebreak(weak: true)"))
      if chapter == 1 then blocks:insert(pandoc.RawBlock("typst", "#overview[")) end
    end
    if block.t == "Header" and pandoc.utils.stringify(block.content) == "Print edition" then
      skip_print_instructions = true
    end
    local navigation = block.t == "Para" and
      pandoc.utils.stringify(block.content):match("^Contents")
    if not skip_print_instructions and not navigation then blocks:insert(block) end
  end
  assert(chapter == 6, "Expected overview, four chapters, and comparison appendix")
  local grouped = pandoc.List()
  local i = 1
  while i <= #blocks do
    local current, following = blocks[i], blocks[i + 1]
    local keep = 0
    if current.t == "Header" and following and following.t == "Table" then
      keep = 2
    elseif current.t == "Header" and following and following.t == "Para"
        and blocks[i + 2] and blocks[i + 2].t == "Table" then
      keep = 3
    elseif current.t == "Para" and following and following.t == "CodeBlock"
        and pandoc.utils.stringify(current.content):match(":$") then
      keep = 2
    end
    if keep > 0 then
      grouped:insert(pandoc.RawBlock("typst", "#block(breakable: false)["))
      for offset = 0, keep - 1 do grouped:insert(blocks[i + offset]) end
      grouped:insert(pandoc.RawBlock("typst", "]"))
      i = i + keep
    elseif current.t == "Para" and following
        and (following.t == "BulletList" or following.t == "OrderedList")
        and pandoc.utils.stringify(current.content):match(":$") then
      -- A list's introduction must not be stranded at the foot of a page.
      grouped:insert(pandoc.RawBlock("typst", "#block(sticky: true)["))
      grouped:insert(current)
      grouped:insert(pandoc.RawBlock("typst", "]"))
      i = i + 1
    else
      grouped:insert(current)
      i = i + 1
    end
  end
  doc.blocks = grouped
  return doc:walk {
    Link = function(link)
      if chapters[link.target] then
        link.target = "#" .. chapters[link.target]
        return link
      end
      -- Source paths remain legible on paper, rather than broken local links.
      if link.target:match("^%.%./%.%./") then
        local path = link.target:gsub("^%.%./%.%./", "")
        local label = pandoc.utils.stringify(link.content)
        if label == path then return pandoc.Code(path) end
        local result = pandoc.Inlines(link.content)
        result:extend {pandoc.Space(), pandoc.Str("("), pandoc.Code(path), pandoc.Str(")")}
        return result
      end
    end,
    Table = function(tbl)
      -- Keep prose tables readable; do not allocate equal widths to short labels.
      if #tbl.colspecs == 3 then
        tbl.colspecs = {{pandoc.AlignLeft, 0.23}, {pandoc.AlignLeft, 0.39}, {pandoc.AlignLeft, 0.38}}
      elseif #tbl.colspecs == 2 then
        tbl.colspecs = {{pandoc.AlignLeft, 0.43}, {pandoc.AlignLeft, 0.57}}
      end
      return tbl
    end,
  }
end
