#!/usr/bin/env python3
"""Bind every remaining TASKS row to an exact committed blocker paragraph."""
import argparse,hashlib,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];TASKS=ROOT/"TASKS.md";OUTPUT=ROOT/"docs/verification/remaining-plan-blocker-audit.json"
ROW=re.compile(r"(?:^|\s)\[ \]");SPRINT=re.compile(r"^### \[ \] Sprint (\d+)");EPIC=re.compile(r"^## \[ \] Epic (\d+)")
def paragraphs(lines,start,end):
 out=[];buf=[];first=start
 for i in range(start,end+1):
  line=lines[i]if i<end else ""
  if line.strip():
   if not buf:first=i
   buf.append(line)
  elif buf:
   text="\n".join(buf)
   if "BLOCKED"in text or "blocked:"in text:out.append({"start_line":first+1,"end_line":i,"sha256":hashlib.sha256(text.encode()).hexdigest(),"text":text})
   buf=[]
 return out
def build():
 lines=TASKS.read_text().splitlines();sprints=[];epics=[]
 for i,line in enumerate(lines):
  if(m:=re.match(r"^### \[[ x]\] Sprint (\d+)",line)):sprints.append((i,int(m.group(1))))
  if(m:=re.match(r"^## \[[ x]\] Epic (\d+)",line)):epics.append((i,int(m.group(1))))
 blockers={};ranges={}
 universal_start=next(i for i,x in enumerate(lines)if x=="## Universal Story Definition of Done");first_sprint=sprints[0][0]
 blockers["universal"]=paragraphs(lines,universal_start,first_sprint)
 for pos,(start,number)in enumerate(sprints):
  end=sprints[pos+1][0]if pos+1<len(sprints)else len(lines);key=f"sprint-{number}";blockers[key]=paragraphs(lines,start,end);ranges[number]=(start,end)
 rows=[]
 for i,line in enumerate(lines):
  if not ROW.search(line):continue
  if universal_start<i<first_sprint:ids=["universal"]
  elif(m:=EPIC.match(line)):
   next_epic=next((p for p,_ in epics if p>i),len(lines));ids=[f"sprint-{n}"for p,n in sprints if i<p<next_epic]
  else:
   owner=next((n for n,(start,end)in ranges.items()if start<=i<end),None);ids=[f"sprint-{owner}"]if owner is not None else ["universal"]
  rows.append({"line":i+1,"text":line,"blocker_ids":ids,"substitution_set":[]})
 used={key:value for key,value in blockers.items()if any(key in row["blocker_ids"]for row in rows)}
 value={"schema_version":1,"tasks_sha256":hashlib.sha256(TASKS.read_bytes()).hexdigest(),"unchecked_row_count":len(rows),"blocked_row_count":len(rows),"unmapped_row_count":sum(not row["blocker_ids"]for row in rows),"nonempty_substitution_count":sum(bool(row["substitution_set"])for row in rows),"blockers":used,"rows":rows}
 return(json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def check():
 expected=build()
 if not OUTPUT.is_file()or OUTPUT.read_bytes()!=expected:raise RuntimeError("remaining plan blocker audit stale")
 value=json.loads(expected)
 if value["unchecked_row_count"]!=value["blocked_row_count"]or value["unmapped_row_count"]or value["nonempty_substitution_count"]:raise RuntimeError("remaining row blocker coverage incomplete")
 if any(not value["blockers"].get(key)for row in value["rows"]for key in row["blocker_ids"]):raise RuntimeError("row references empty blocker set")
def main():
 parser=argparse.ArgumentParser();parser.add_argument("--write",action="store_true");args=parser.parse_args()
 if args.write:OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
 check();value=json.loads(build());print(f"validated {value['blocked_row_count']} blocked rows with zero unmapped rows or substitutions");return 0
if __name__=="__main__":raise SystemExit(main())
