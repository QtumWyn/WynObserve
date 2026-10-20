defmodule AetherTest do
  use ExUnit.Case
  doctest Aether

  test "greets the world" do
    assert Aether.hello() == :world
  end
end
