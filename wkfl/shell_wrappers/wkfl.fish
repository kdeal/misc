function wkfl --wraps wkfl
    set -l actions_file (mktemp)
    command wkfl --shell-actions-file "$actions_file" $argv
    for line in (cat $actions_file)
        set -l action (string split "," $line)
        switch $action[1]
            case "cd"
                cd "$action[2]"
            case "edit_file"
                eval $EDITOR "$action[2]"
            case "*"
                echo "Unhandled action: $action[1]"
        end
    end
    rm "$actions_file"
end
